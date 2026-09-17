import React, { useCallback, useEffect, useState } from 'react';
import { RegistrarStatus, TaskItem } from './types';

const API_ORIGIN = 'http://127.0.0.1:8080';
type DashboardState = 'loading' | 'available' | 'unavailable';

function label(status: TaskItem['status']) {
  return { Pending: 'Awaiting dispatch', Claimed: 'Generating draft', Review: 'Awaiting owner review', Verified: 'Verification recorded', Failed: 'Generation failed' }[status];
}

export const App: React.FC = () => {
  const [state, setState] = useState<DashboardState>('loading');
  const [tasks, setTasks] = useState<TaskItem[]>([]);
  const [status, setStatus] = useState<RegistrarStatus | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [updated, setUpdated] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const controller = new AbortController();
    const timeout = window.setTimeout(() => controller.abort(), 4_000);
    try {
      const [statusResponse, tasksResponse, logsResponse] = await Promise.all([
        fetch(`${API_ORIGIN}/api/status`, { signal: controller.signal }),
        fetch(`${API_ORIGIN}/api/tasks`, { signal: controller.signal }),
        fetch(`${API_ORIGIN}/api/logs`, { signal: controller.signal }),
      ]);
      if (!statusResponse.ok || !tasksResponse.ok || !logsResponse.ok) throw new Error('registrar API error');
      const nextStatus = await statusResponse.json() as RegistrarStatus;
      const nextTasks = await tasksResponse.json() as TaskItem[];
      const nextLogs = await logsResponse.json() as { log: string };
      setStatus(nextStatus);
      setTasks(nextTasks);
      setLogs(nextLogs.log.split('\n').map(line => line.trim()).filter(Boolean).slice(-30).reverse());
      setUpdated(new Date().toLocaleTimeString());
      setState('available');
    } catch {
      setState('unavailable');
    } finally {
      window.clearTimeout(timeout);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 30_000);
    return () => window.clearInterval(interval);
  }, [refresh]);

  return (
    <main className="min-h-screen bg-slate-950 p-6 font-sans text-slate-100">
      <div className="mx-auto max-w-5xl space-y-6">
        <header className="rounded-3xl border border-slate-800 bg-slate-900 p-8 shadow-2xl">
          <div className="flex flex-col items-start justify-between gap-4 md:flex-row md:items-center">
            <div>
              <h1 className="text-2xl font-black text-white">🛠️ Aaroneous DevTools</h1>
              <p className="mt-2 text-sm text-slate-400">A read-only view of local draft generation and the owner-review queue. This dashboard cannot apply or commit product work.</p>
            </div>
            <button onClick={() => void refresh()} className="rounded-xl border border-slate-700 bg-slate-950 px-4 py-2 text-sm font-bold text-slate-200 hover:border-emerald-500">Refresh</button>
          </div>
          {state === 'unavailable' && <p className="mt-5 rounded-xl border border-amber-500/30 bg-amber-950/30 p-4 text-sm text-amber-100">The loopback registrar API at <code>{API_ORIGIN}</code> did not respond. No state is inferred and no work was started.</p>}
        </header>

        {state === 'available' && status && <section className="grid grid-cols-2 gap-3 md:grid-cols-5">
          {[['Tasks', status.total_tasks], ['Awaiting dispatch', status.pending], ['Owner review', status.review], ['Verified', status.verified], ['Failed', status.failed]].map(([name, value]) =>
            <div key={String(name)} className="rounded-2xl border border-slate-800 bg-slate-900 p-4"><div className="text-2xl font-black">{value}</div><div className="text-xs font-bold text-slate-400">{name}</div></div>
          )}
        </section>}

        <section className="grid grid-cols-1 gap-6 lg:grid-cols-3">
          <div className="space-y-4 rounded-2xl border border-slate-800 bg-slate-900 p-6 lg:col-span-2">
            <div className="border-b border-slate-800 pb-3"><h2 className="text-lg font-bold">Registered work</h2><p className="text-xs text-slate-400">Registrar queue state{updated ? ` · updated ${updated}` : ''}</p></div>
            {state === 'available' && tasks.length === 0 && <p className="rounded-xl bg-slate-950 p-4 text-sm text-slate-400">No registered tasks are available.</p>}
            {tasks.map(task => <article key={task.id} className="flex items-center justify-between gap-4 rounded-xl border border-slate-800 bg-slate-950 p-4">
              <div className="min-w-0"><div className="font-bold text-white">{task.task}</div><p className="mt-1 text-xs text-emerald-300">{task.crate_name} · {task.target_file}</p><p className="mt-1 text-xs text-slate-500">{task.instruction}</p></div>
              <span className="whitespace-nowrap rounded-full border border-blue-500/20 bg-blue-500/10 px-3 py-1 text-xs font-bold text-blue-300">{label(task.status)}</span>
            </article>)}
          </div>
          <aside className="space-y-4 rounded-2xl border border-slate-800 bg-slate-900 p-6"><div><h2 className="text-lg font-bold">Registrar activity</h2><p className="text-xs text-slate-400">Recorded events only; drafts remain pending owner review.</p></div><div className="max-h-96 space-y-2 overflow-y-auto rounded-xl border border-slate-800 bg-slate-950 p-4 font-mono text-xs text-slate-300">{state === 'available' && logs.length === 0 && <p className="text-slate-500">No recorded activity.</p>}{logs.map((entry, index) => <p key={`${entry}-${index}`} className="border-b border-slate-900 pb-2">{entry}</p>)}</div></aside>
        </section>
      </div>
    </main>
  );
};
