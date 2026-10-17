"""Isolated fault fixtures. Never opens or controls the live board."""
import hashlib
import io
import json
import sys
if sys.platform == "win32":
    import msvcrt
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import board
import recovery

STAGE = Path(__file__).resolve().parent
PAYLOAD = {"actor":"Owner", "kind":"evidence", "text":"retained test record", "evidence":"isolated fixture"}

class RecoveryTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp(prefix='fixture-', dir=STAGE))
        (self.root/'session.json').write_text(json.dumps({"lifecycle":{"state":"open","closes_at":None}}))
        (self.root/'board-control.json').write_text('{"state":"stop"}')
        (self.root/'board-host.lock').write_bytes(b'0')
        self.instance = board.Board(self.root,lambda:1000,{"max_storage_bytes":32768})

    def tearDown(self):
        # Retain exact fixture evidence for review; no recursive cleanup.
        pass

    def hashes(self):
        return {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in self.instance.entries.iterdir() if p.is_file()}

    def fail_before(self):
        with patch.object(board.os,'link',side_effect=OSError('pre-publication fault')):
            with self.assertRaises(board.PublicationFailure):
                self.instance.append(PAYLOAD)

    def fail_after(self):
        unlink = Path.unlink
        def fail(path,*args,**kwargs):
            if path.parent == self.instance.entries and path.name.endswith('.pending'):
                raise OSError('cleanup fault')
            return unlink(path,*args,**kwargs)
        with patch.object(Path,'unlink',fail):
            with self.assertRaises(board.PublicationFailure):
                self.instance.append(PAYLOAD)

    def restart(self):
        return board.Board(self.root,lambda:1000,{"max_storage_bytes":32768})

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_before_publication_blocks_replay_and_recovery(self):
        self.fail_before()
        before = self.hashes()
        plan = recovery.inspect_pending(self.root)
        self.assertEqual(plan['records'][0]['classification'],'unpublished')
        restarted = self.restart()
        self.assertEqual(restarted.count,0)
        self.assertFalse(restarted.snapshot()['intake_open'])
        with self.assertRaises(OSError): restarted.append(PAYLOAD)
        with self.assertRaises(ValueError): recovery.reconcile_matching(self.root,recovery.plan_digest(plan))
        self.assertEqual(before,self.hashes())

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_matching_publication_reconciles_counters_and_preserves_evidence(self):
        self.fail_after()
        self.assertEqual(self.instance.count,1)
        restarted = self.restart()
        self.assertEqual(restarted.count,1)
        self.assertEqual(len(restarted.snapshot()['entries']),1)
        self.assertFalse(restarted.snapshot()['intake_open'])
        plan = recovery.inspect_pending(self.root)
        self.assertEqual(plan['records'][0]['classification'],'published-matching')
        published = next(self.instance.entries.glob('*.json'))
        original = published.read_bytes()
        outcome = recovery.reconcile_matching(self.root,recovery.plan_digest(plan))
        self.assertFalse(outcome['retry_allowed'])
        self.assertEqual(published.read_bytes(),original)
        self.assertEqual(next((self.root/'recovery-evidence').iterdir()).read_bytes(),original)
        self.assertFalse(list(self.instance.entries.glob('.*.pending')))
        clean = self.restart()
        self.assertEqual(clean.count,1)
        self.assertEqual(clean.last_id,1)
        self.assertEqual(clean.used_bytes,len(original))
        self.assertTrue(clean.snapshot()['intake_open'])
        self.assertEqual(clean.append(dict(PAYLOAD,text='explicit new record'))['id'],2)

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_conflicting_bytes_stay_blocked(self):
        self.fail_after()
        pending = next(self.instance.entries.glob('.*.pending'))
        obj=json.loads(pending.read_bytes()); obj['text']='conflict'
        # The original publication creates hard links. Replace only the fixture
        # pending name, rather than inadvertently modifying both linked names.
        pending.unlink(); pending.write_text(json.dumps(obj))
        before=self.hashes(); plan=recovery.inspect_pending(self.root)
        self.assertEqual(plan['records'][0]['classification'],'conflicting-or-invalid')
        with self.assertRaises(ValueError): recovery.reconcile_matching(self.root,recovery.plan_digest(plan))
        self.assertEqual(before,self.hashes())

    def test_partial_and_duplicate_json_are_invalid(self):
        self.fail_before()
        pending=next(self.instance.entries.glob('.*.pending'))
        for raw in (b'{',b'{"id":1,"id":1}'):
            pending.write_bytes(raw)
            self.assertEqual(recovery.inspect_pending(self.root)['records'][0]['classification'],'conflicting-or-invalid')
            self.assertFalse(self.restart().snapshot()['intake_open'])

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_stale_plan_refused(self):
        self.fail_after(); plan=recovery.inspect_pending(self.root)
        pending=next(self.instance.entries.glob('.*.pending')); pending.write_bytes(b'changed')
        before=self.hashes()
        with self.assertRaisesRegex(ValueError,'stale'): recovery.reconcile_matching(self.root,recovery.plan_digest(plan))
        self.assertEqual(before,self.hashes())

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_lease_and_stop_required(self):
        self.fail_after(); digest=recovery.plan_digest(recovery.inspect_pending(self.root)); before=self.hashes()
        (self.root/'board-control.json').write_text('{"state":"run"}')
        with self.assertRaises(ValueError): recovery.reconcile_matching(self.root,digest)
        (self.root/'board-control.json').write_text('{"state":"stop"}')
        with (self.root/'board-host.lock').open('r+b') as handle:
            msvcrt.locking(handle.fileno(),msvcrt.LK_NBLCK,1)
            try:
                with self.assertRaises(OSError): recovery.reconcile_matching(self.root,digest)
            finally:
                handle.seek(0); msvcrt.locking(handle.fileno(),msvcrt.LK_UNLCK,1)
        self.assertEqual(before,self.hashes())

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_archive_failure_retains_pending_and_published(self):
        self.fail_after(); plan=recovery.inspect_pending(self.root); before=self.hashes(); fsync=recovery.os.fsync
        with patch.object(recovery.os,'fsync',side_effect=OSError('archive flush failure')):
            with self.assertRaises(OSError): recovery.reconcile_matching(self.root,recovery.plan_digest(plan))
        self.assertEqual(before,self.hashes())
        # Retained complete bytes can be verified on a later explicit recovery.
        recovery.reconcile_matching(self.root,recovery.plan_digest(recovery.inspect_pending(self.root)))
        self.assertEqual(self.restart().count,1)

    @unittest.skipUnless(sys.platform == "win32", "ordinary host lease is Windows-only")
    def test_conflicting_archive_never_overwritten(self):
        self.fail_after(); plan=recovery.inspect_pending(self.root); row=plan['records'][0]
        archive=self.root/'recovery-evidence'; archive.mkdir()
        retained=archive/(row['pending']+'.'+row['sha256']+'.retained'); retained.write_bytes(b'original unrelated evidence')
        before=self.hashes()
        with self.assertRaises(ValueError): recovery.reconcile_matching(self.root,recovery.plan_digest(plan))
        self.assertEqual(retained.read_bytes(),b'original unrelated evidence'); self.assertEqual(before,self.hashes())

    def test_duplicate_pending_identity_blocked(self):
        self.fail_after(); pending=next(self.instance.entries.glob('.*.pending'))
        (pending.parent/'.0001-aaaaaaaaaaaa.json.pending').write_bytes(pending.read_bytes())
        plan=recovery.inspect_pending(self.root)
        self.assertTrue(all(r['classification']=='conflicting-or-invalid' for r in plan['records']))

    def test_readable_lifecycle_and_capacity_remain_truthful(self):
        self.instance.append(PAYLOAD)
        for state,deadline in [('paused',None),('closed',None),('open',900)]:
            (self.root/'session.json').write_text(json.dumps({'lifecycle':{'state':state,'closes_at':deadline}}))
            snap=self.instance.snapshot(); self.assertFalse(snap['intake_open']); self.assertEqual(len(snap['entries']),1)
        (self.root/'session.json').write_text(json.dumps({'lifecycle':{'state':'open','closes_at':None}}))
        self.instance.config['max_storage_bytes']=self.instance.used_bytes
        self.assertTrue(self.instance.snapshot()['intake_open'])
        with self.assertRaisesRegex(ValueError,'storage budget'): self.instance.append(PAYLOAD)

    def test_http_failure_contains_matching_publication_without_retry_permission(self):
        from email.message import Message
        handler=object.__new__(board.handler_for(self.instance,'http://127.0.0.1:31492'))
        handler.path='/api/entries'; handler.headers=Message()
        raw=json.dumps(PAYLOAD).encode(); handler.rfile=io.BytesIO(raw)
        for k,v in {'Host':'127.0.0.1:31492','Origin':'http://127.0.0.1:31492','X-Board-Token':self.instance.token,'Content-Type':'application/json','Content-Length':str(len(raw))}.items(): handler.headers[k]=v
        replies=[]; handler.respond=lambda status,body:replies.append((status,body))
        unlink=Path.unlink
        def fail(path,*a,**k):
            if path.parent==self.instance.entries and path.name.endswith('.pending'): raise OSError('cleanup fault')
            return unlink(path,*a,**k)
        with patch.object(Path,'unlink',fail): handler.do_POST()
        status,body=replies[0]; self.assertEqual(status,503); self.assertFalse(body['retry_allowed'])
        self.assertEqual(body['publication_recovery']['records'][0]['classification'],'published-matching')
        self.assertEqual(self.instance.count,1)

if __name__=='__main__':
    unittest.main(verbosity=2)
