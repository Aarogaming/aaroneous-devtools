'use strict';
const el = id => document.getElementById(id);
const fragment = location.hash.slice(1);
if (fragment) { sessionStorage.setItem('devtools-token', fragment); history.replaceState(null, '', '/'); }
const token = sessionStorage.getItem('devtools-token') || '';
let snapshot = null;
let busy = false;
let actionQueue = Promise.resolve();
function notice(message, error = false) { el('notice').textContent = message; el('notice').classList.toggle('error', error); }
async function api(path, data) {
  const options = {headers: {'X-Devtools-Token': token}};
  if (data !== undefined) { options.method = 'POST'; options.headers['Content-Type'] = 'application/json'; options.body = JSON.stringify(data); }
  const response = await fetch(path, options);
  const value = await response.json();
  if (!response.ok) throw new Error(value.error || 'The operation failed');
  return value;
}
function list(container, title, values) {
  if (!Array.isArray(values) || !values.length) return;
  const heading = document.createElement('h3'); heading.textContent = title;
  const items = document.createElement('ul');
  for (const value of values) { const item = document.createElement('li'); item.textContent = typeof value === 'string' ? value : JSON.stringify(value); items.append(item); }
  container.append(heading, items);
}
function controls() {
  const build = snapshot?.plan.mode === 'build';

  el('research-button').disabled = busy || !!snapshot?.active;
  el('blueprint-button').disabled = busy || build;
  el('refine-button').disabled = busy || build || !snapshot?.plan.blueprint;
  el('mode-button').disabled = busy || (!build && !snapshot?.plan.build_ready);
  el('build-button').disabled = busy || !build;
}
async function refresh() {
  snapshot = await api('/api/status');
  const {plan, status, research, active, last_run} = snapshot;
  el('mode-badge').textContent = plan.mode === 'build' ? 'Job: Build' : 'Job: Plan';
  el('workspace').textContent = `${status.checkout} · ${status.local_model} · ${status.results.length} support results awaiting maintainer review`;
  el('readiness-title').textContent = plan.build_ready ? 'Job publication checks passed' : 'Job stays in Plan';
  el('readiness-summary').textContent = plan.packet.summary;
  el('criteria').replaceChildren();
  for (const criterion of plan.criteria) {
    const item = document.createElement('li'); item.className = criterion.satisfied ? 'pass' : 'blocked';
    const title = document.createElement('strong'); title.textContent = `${criterion.satisfied ? '✓' : '○'} ${criterion.label}`;
    const detail = document.createElement('span'); detail.textContent = criterion.detail;
    item.append(title, detail); el('criteria').append(item);
  }
  el('mode-button').textContent = plan.mode === 'build' ? 'Return to Plan' : 'Switch to Build';
  el('blueprint').replaceChildren();
  if (plan.blueprint) {
    const b = plan.blueprint;
    el('blueprint-purpose').textContent = b.purpose;
    list(el('blueprint'), 'Scope', b.scope);
    list(el('blueprint'), 'Prepared steps', b.steps.map((s, i) => `${i + 1}. ${s.title} — ${s.expected_evidence}`));
    list(el('blueprint'), 'Success criteria', b.success_criteria);
    list(el('blueprint'), 'Rollback strategy', [b.rollback]);
    list(el('blueprint'), 'Open questions', b.unresolved_questions);
  }
  el('research-summary').textContent = research.summary;
  el('research').replaceChildren();
  list(el('research'), 'Unverified observations', research.observations);
  list(el('research'), 'Risks and questions', research.questions);
  list(el('research'), 'Suggested acceptance tests', research.suggested_tests);
  if (research.error) list(el('research'), 'Failure evidence', [research.error]);
  el('operation').textContent = active ? `${active.kind === 'verification' ? 'Prepared checks' : 'Read-only research'} is running.` : last_run?.meaning || 'No operation recorded.';
  el('evidence').textContent = JSON.stringify({repository_status: status.repository_status, worker_observation: status.worker_observation, worker_status_stale: status.worker_status_stale, reservation_present: status.reservation_present, last_run}, null, 2);
  controls();
}
function action(work) {
  actionQueue = actionQueue.then(async () => {
    busy = true; controls();
    try { const message = await work(); await refresh(); if (message) notice(message); }
    catch (error) { notice(error.message, true); }
    finally { busy = false; controls(); }
  });
  return actionQueue;
}
for (const button of document.querySelectorAll('[data-packet]')) button.onclick = () => action(async () => (await api('/api/packet', {packet:button.dataset.packet})).message);
el('mode-button').onclick = () => action(async () => (await api('/api/mode', {mode:snapshot.plan.mode === 'build' ? 'plan' : 'build'})).message);
if (!token) notice('Open the complete link printed by the utility to connect this local session.', true);
else action(async () => { await refresh(); return 'Connected. Job authoring starts in Plan; published jobs run independently.'; });
setInterval(() => { if (!busy && token) action(async () => { await refresh(); }); }, 10000);
