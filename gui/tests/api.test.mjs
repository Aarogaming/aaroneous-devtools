import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parseSnapshot, loadSnapshot, createRefreshController } from '../src/api.ts';

const status = { checkout: 'checkout', model: 'local', endpoint: 'http://127.0.0.1:11434', total_tasks: 1, pending: 0, review: 1, verified: 0, failed: 0 };
const task = { id: 'T1', crate_name: 'wire', task: 'Review', target_file: 'src/lib.rs', instruction: 'Inspect', status: 'Review' };
const snapshot = parseSnapshot(status, [task], { log: 'one\ntwo\n' });

test('parses review state and retains only the latest 30 nonempty log lines', () => {
  const parsed = parseSnapshot(status, [task], { log: Array.from({ length: 40 }, (_, index) => `${index}`).join('\n') });
  assert.equal(parsed.tasks[0].status, 'Review');
  assert.equal(parsed.logs.length, 30);
  assert.equal(parsed.logs[0], '39');
  assert.equal(parsed.logs[29], '10');
});

test('rejects malformed responses, unknown states, duplicate IDs and invalid counts', () => {
  for (const [s, t, l] of [
    [null, [task], { log: '' }],
    [status, {}, { log: '' }],
    [status, [null], { log: '' }],
    [status, [{ ...task, task: {} }], { log: '' }],
    [status, [{ ...task, status: 'completed' }], { log: '' }],
    [status, [task, task], { log: '' }],
    [{ ...status, verified: -1 }, [task], { log: '' }],
    [{ ...status, review: 1.5 }, [task], { log: '' }],
    [status, [task], { log: null }],
  ]) assert.throws(() => parseSnapshot(s, t, l));
});

test('failed HTTP or malformed JSON cannot publish a partial snapshot', async () => {
  await assert.rejects(loadSnapshot('http://127.0.0.1:8080', new AbortController().signal, async () => new Response('{}', { status: 500 })));
  await assert.rejects(loadSnapshot('http://127.0.0.1:8080', new AbortController().signal, async () => new Response('broken json')));
});

test('uses all three read-only endpoints and validates them as one snapshot', async () => {
  const calls = [];
  const result = await loadSnapshot('http://127.0.0.1:8080', new AbortController().signal, async (url, options) => {
    calls.push([url, options]);
    const payload = url.endsWith('/status') ? status : url.endsWith('/tasks') ? [task] : { log: 'one\ntwo\n' };
    return new Response(JSON.stringify(payload));
  });
  assert.deepEqual(result, snapshot);
  assert.equal(calls.length, 3);
  assert.ok(calls.every(([, options]) => options.cache === 'no-store' && !options.method));
});

test('oversized responses are rejected before JSON parsing', async () => {
  await assert.rejects(
    loadSnapshot('http://127.0.0.1:8080', new AbortController().signal, async () => new Response('x'.repeat(1_048_577))),
    /exceeds 1 MiB/,
  );
});

test('overlapping refreshes are suppressed and failed refreshes retain the last snapshot', async () => {
  let resolve;
  let attempts = 0;
  let retained;
  let errors = 0;
  const busy = [];
  const controller = createRefreshController(() => {
    attempts++;
    return attempts === 1 ? new Promise(done => { resolve = done; }) : Promise.reject(new Error('offline'));
  }, value => { retained = value; }, () => { errors++; }, value => busy.push(value));
  const first = controller.refresh();
  await controller.refresh();
  assert.equal(attempts, 1);
  resolve(snapshot);
  await first;
  await controller.refresh();
  assert.equal(retained, snapshot);
  assert.equal(errors, 1);
  assert.deepEqual(busy, [true, false, true, false]);
  controller.stop();
});

test('stopping cancels pending work and suppresses late callbacks', async () => {
  let resolve;
  let signal;
  let published = 0;
  const busy = [];
  const controller = createRefreshController(input => {
    signal = input;
    return new Promise(done => { resolve = done; });
  }, () => { published++; }, () => assert.fail('stopped controller reported an error'), value => busy.push(value));
  const pending = controller.refresh();
  controller.stop();
  assert.equal(signal.aborted, true);
  resolve(snapshot);
  await pending;
  await controller.refresh();
  assert.equal(published, 0);
  assert.deepEqual(busy, [true]);
});

test('deadline abort reports failure and allows a later successful refresh', async () => {
  let attempts = 0;
  let errors = 0;
  let published = 0;
  const controller = createRefreshController(signal => {
    attempts++;
    if (attempts > 1) return Promise.resolve(snapshot);
    return new Promise((_, reject) => signal.addEventListener('abort', () => reject(new Error('timed out')), { once: true }));
  }, () => { published++; }, () => { errors++; }, () => {}, 5);
  await controller.refresh();
  assert.equal(errors, 1);
  await controller.refresh();
  assert.equal(published, 1);
  controller.stop();
});
