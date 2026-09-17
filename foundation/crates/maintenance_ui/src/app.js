'use strict';
import {createApiClient, validateSnapshot} from './client.mjs';
const el = id => document.getElementById(id);
const fragment = location.hash.slice(1);
if (fragment) { sessionStorage.setItem('devtools-token', fragment); history.replaceState(null, '', '/'); }
const token = sessionStorage.getItem('devtools-token') || '';
let snapshot = null;
let busy = false;
let actionQueue = Promise.resolve();
let selectedResult = null;
let connected = false;
let lastObserved = null;
const client = createApiClient(token);
function notice(message, error = false) { el('notice').textContent = message; el('notice').classList.toggle('error', error); }
async function api(path, data) {
  try { return await client.request(path, data); }
  catch (error) { if (error.unknownOutcome) connected = false; throw error; }
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
  for (const button of document.querySelectorAll('[data-packet]')) button.disabled = busy;
  el('research-button').disabled = busy || !!snapshot?.active;
  el('blueprint-button').disabled = busy || build;
  el('refine-button').disabled = busy || build || !snapshot?.plan.blueprint;
  el('mode-button').disabled = busy || (!build && !snapshot?.plan.build_ready);
  el('build-button').disabled = busy || !build;
  for (const button of document.querySelectorAll('[data-result], [data-disposition]')) button.disabled = busy || !!snapshot?.active;
  for (const button of document.querySelectorAll('[data-disposition]')) button.disabled ||= !selectedResult;
  if (!connected) for (const button of document.querySelectorAll('[data-packet], [data-result], [data-disposition], #mode-button')) button.disabled = true;
  el('refresh-button').disabled = busy || !token;
  el('connection').textContent = connected ? `Connected · last observed ${lastObserved}` : `Unavailable · ${lastObserved ? `retained information is STALE (last observed ${lastObserved})` : 'no state observed'} · actions disabled`;
}
async function refresh() {
  try { snapshot = validateSnapshot(await api('/api/status')); }
  catch (error) { connected = false; controls(); throw error; }
  connected = true;
  lastObserved = new Date().toLocaleTimeString();
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
  const pending = snapshot.review_results || [];
  if (selectedResult && !pending.some(result => result.filename === selectedResult)) {
    selectedResult = null;
    el('result-detail').textContent = 'Select a pending result to inspect its complete evidence.';
    el('review-note').value = '';
  }
  el('pending-results').replaceChildren();
  for (const result of pending) {
    const button = document.createElement('button');
    button.dataset.result = result.filename;
    button.textContent = `${result.job_id} — ${result.outcome}`;
    button.onclick = () => action(async () => {
      const record = await api('/api/result', {filename: result.filename});
      selectedResult = result.filename;
      el('result-detail').textContent = JSON.stringify(record, null, 2);
      return 'Evidence opened. Model output remains unverified; record your own review decision.';
    });
    el('pending-results').append(button);
  }
  el('review-summary').textContent = pending.length ? `${pending.length} utility results need review.` : 'No utility results awaiting review. Other worker results remain outside this interface.';
  el('research').replaceChildren();
  if (research.review) list(el('research'), 'Retained review decision', [`${research.review.disposition}: ${research.review.note}`]);
  list(el('research'), 'Unverified observations', research.observations);
  list(el('research'), 'Risks and questions', research.questions);
  list(el('research'), 'Suggested acceptance tests', research.suggested_tests);
  if (research.error) list(el('research'), 'Failure evidence', [research.error]);
  el('operation').textContent = active ? `${active.kind === 'verification' ? 'Prepared checks' : 'Read-only research'} is running.` : last_run?.meaning || 'No operation recorded.';
  el('evidence').textContent = JSON.stringify({repository_status: status.repository_status, worker_observation: status.worker_observation, worker_status_stale: status.worker_status_stale, reservation_present: status.reservation_present, last_run}, null, 2);
  controls();
  notice('Connected. Results remain unverified; review decisions never authorize product edits.');
}
function action(work, refreshAfter = true) {
  actionQueue = actionQueue.then(async () => {
    busy = true; controls();
    try { const message = await work(); if (refreshAfter) await refresh(); if (message) notice(message); }
    catch (error) { notice(error.message, true); }
    finally { busy = false; controls(); }
  });
  return actionQueue;
}
for (const button of document.querySelectorAll('[data-packet]')) button.onclick = () => action(async () => (await api('/api/packet', {packet:button.dataset.packet})).message);
el('refresh-button').onclick = () => action(refresh, false);
el('mode-button').onclick = () => action(async () => (await api('/api/mode', {mode:snapshot.plan.mode === 'build' ? 'plan' : 'build'})).message);
for (const button of document.querySelectorAll('[data-disposition]')) button.onclick = () => action(async () => {
  if (!selectedResult || !el('review-note').value.trim()) throw new Error('Inspect a result and enter a review note first.');
  return (await api('/api/review', {filename: selectedResult, disposition: button.dataset.disposition, note: el('review-note').value})).message;
});
if (!token) notice('Open the complete link printed by the utility to connect this local session.', true);
else action(refresh, false);
setInterval(() => { if (!busy && token) action(refresh, false); }, 10000);
window.addEventListener('pagehide', () => client.stop());
controls();
