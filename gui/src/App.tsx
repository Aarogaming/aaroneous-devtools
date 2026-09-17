import React, { useEffect, useRef, useState } from 'react';
import { RegistrarStatus, TaskItem } from './types';
import { createRefreshController, loadSnapshot } from './api';

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
  const [refreshing, setRefreshing] = useState(false);
  const controllerRef = useRef<ReturnType<typeof createRefreshController> | null>(null);

  useEffect(() => {
    const controller = createRefreshController(
      signal => loadSnapshot(API_ORIGIN, signal),
      snapshot => {
        setStatus(snapshot.status);
        setTasks(snapshot.tasks);
        setLogs(snapshot.logs);
        setUpdated(new Date().toLocaleTimeString());
        setState('available');
      },
      () => setState('unavailable'),
      setRefreshing,
    );
    controllerRef.current = controller;
    void controller.refresh();
    const interval = window.setInterval(() => void controller.refresh(), 30_000);
    return () => {
      window.clearInterval(interval);
      controller.stop();
      if (controllerRef.current === controller) controllerRef.current = null;
    };
  }, []);

  return (
    <main className="min-h-screen bg-slate-950 p-6 font-sans text-slate-100">
      <div className="mx-auto max-w-5xl space-y-6">
        <header className="rounded-3xl border border-slate-800 bg-slate-900 p-8 shadow-2xl">
          <div className="flex flex-col items-start justify-between gap-4 md:flex-row md:items-center">
            <div>
              <h1 className="text-2xl font-black text-white">🛠️ Aaroneous DevTools</h1>
              <p className="mt-2 text-sm text-slate-400">A read-only view of local draft generation and the owner-review queue. This dashboard cannot apply or commit product work.</p>
            </div>
            <button disabled={refreshing} onClick={() => void controllerRef.current?.refresh()} className="rounded-xl border border-slate-700 bg-slate-950 px-4 py-2 text-sm font-bold text-slate-200 hover:border-emerald-500 disabled:opacity-50">{refreshing ? 'Refreshing…' : 'Refresh'}</button>
          </div>
          {state === 'loading' && <p role="status" className="mt-5 text-sm text-slate-400">Checking registrar status…</p>}
          {state === 'unavailable' && <p role="alert" className="mt-5 rounded-xl border border-amber-500/30 bg-amber-950/30 p-4 text-sm text-amber-100">Status could not be refreshed: the registrar is unavailable or returned invalid data. {updated ? `The information below is stale; last observed at ${updated}.` : 'No registrar information is available.'} No work was started.</p>}
        </header>

        {status && <section aria-label={state === 'unavailable' ? 'Stale task counts' : 'Observed task counts'} className="grid grid-cols-2 gap-3 md:grid-cols-5">
          {[['Tasks', status.total_tasks], ['Awaiting dispatch', status.pending], ['Owner review', status.review], ['Verified', status.verified], ['Failed', status.failed]].map(([name, value]) =>
            <div key={String(name)} className="rounded-2xl border border-slate-800 bg-slate-900 p-4"><div className="text-2xl font-black">{value}</div><div className="text-xs font-bold text-slate-400">{name}</div></div>
          )}
        </section>}

        <section className="grid grid-cols-1 gap-6 lg:grid-cols-3">
          <div className="space-y-4 rounded-2xl border border-slate-800 bg-slate-900 p-6 lg:col-span-2">
            <div className="border-b border-slate-800 pb-3"><h2 className="text-lg font-bold">Registered work</h2><p className="text-xs text-slate-400">{state === 'unavailable' && updated ? 'Stale registrar queue' : 'Registrar queue state'}{updated ? ` · observed ${updated}` : ''}</p></div>
            {state === 'available' && tasks.length === 0 && <p className="rounded-xl bg-slate-950 p-4 text-sm text-slate-400">No registered tasks are available.</p>}
            {tasks.map(task => <article key={task.id} className="flex flex-col items-start justify-between gap-4 rounded-xl border border-slate-800 bg-slate-950 p-4 sm:flex-row sm:items-center">
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
