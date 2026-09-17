# DevTools dashboard

This React client presents observed registrar tasks and activity. It uses read-only requests to `http://127.0.0.1:8080` and cannot dispatch, promote, apply or commit product work. The registrar remains a separately configured Rust service.

From this directory, run `npm ci`, then `npm run build`. The build runs the regression tests, TypeScript checks and Vite production build. Tailwind CSS is bundled locally; the page loads no remote styling script. Use `npm run preview -- --host 127.0.0.1` to inspect the built page locally, or `npm run dev -- --host 127.0.0.1` for development.

Refreshes occur every 30 seconds, with a four-second timeout and one active refresh at a time. Each endpoint response is bounded to 1 MiB; task shapes, states, IDs, counts and log data are validated before replacing the displayed snapshot. Invalid data or unavailable services produce an explicit error state. Last observed data is retained and marked stale; it is never represented as current health or new verification.

The regression suite covers malformed payloads, unknown task states, duplicate IDs, invalid counts, oversized responses, HTTP/JSON failures, overlapping refreshes, cancellation and timeout recovery. `npm test` runs it separately on Node 22.6 or later. Git-triggered Dashboard CI runs the same production build on Linux and Windows.
