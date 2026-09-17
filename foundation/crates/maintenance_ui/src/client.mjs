export function validateSnapshot(value) {
  const p = value?.plan, s = value?.status, r = value?.research;
  if (!p || !s || !r || !['plan', 'build'].includes(p.mode) || typeof p.build_ready !== 'boolean'
    || typeof p.packet?.summary !== 'string' || !Array.isArray(p.criteria)
    || !p.criteria.every(c => typeof c.label === 'string' && typeof c.detail === 'string' && typeof c.satisfied === 'boolean')
    || typeof s.checkout !== 'string' || typeof s.local_model !== 'string' || !Array.isArray(s.results)
    || typeof r.summary !== 'string' || !Array.isArray(value.review_results)
    || !value.review_results.every(x => typeof x.filename === 'string' && typeof x.job_id === 'string' && typeof x.outcome === 'string')
    || (p.blueprint && (!Array.isArray(p.blueprint.steps) || !p.blueprint.steps.every(x => typeof x.title === 'string' && typeof x.expected_evidence === 'string')))) {
    throw new Error('Invalid status response; no controls were enabled.');
  }
  return value;
}

export function createApiClient(token, {fetchImpl = fetch, timeoutMs = 6000, maxBytes = 2097152} = {}) {
  const pending = new Set();
  return {
    async request(path, data) {
      const controller = new AbortController();
      pending.add(controller);
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      const mutation = data !== undefined;
      let reader;
      try {
        const options = {headers: {'X-Devtools-Token': token}, signal: controller.signal, cache: 'no-store'};
        if (mutation) { options.method = 'POST'; options.headers['Content-Type'] = 'application/json'; options.body = JSON.stringify(data); }
        const response = await fetchImpl(path, options);
        if (!response.ok) {
          if (mutation && response.status >= 500) throw new Error('Server failed; mutation may have completed');
          const error = new Error(`Request rejected (${response.status}). Reconnect and inspect current state.`);
          error.rejected = true;
          throw error;
        }
        if (!response.body) throw new Error('Missing response body');
        reader = response.body.getReader();
        const decoder = new TextDecoder('utf-8', {fatal: true});
        let size = 0, text = '';
        for (;;) {
          const {value, done} = await reader.read();
          if (done) break;
          size += value.byteLength;
          if (size > maxBytes) throw new Error('Response exceeds size limit');
          text += decoder.decode(value, {stream: true});
        }
        text += decoder.decode();
        if (controller.signal.aborted) throw new Error('Request cancelled');
        return JSON.parse(text);
      } catch (cause) {
        if (cause.rejected) throw cause;
        const error = new Error(mutation
          ? 'Operation outcome is unknown. Reconnect and inspect retained evidence before repeating it.'
          : 'Connection unavailable or invalid response. Retained information is stale; actions are disabled.');
        error.unknownOutcome = mutation;
        throw error;
      } finally {
        clearTimeout(timer);
        if (reader) { await reader.cancel().catch(() => {}); reader.releaseLock(); }
        pending.delete(controller);
      }
    },
    stop() { for (const controller of pending) controller.abort(); }
  };
}
