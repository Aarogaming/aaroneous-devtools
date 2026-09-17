import type { RegistrarStatus, TaskItem } from './types.ts';

export interface DashboardSnapshot {
  status: RegistrarStatus;
  tasks: TaskItem[];
  logs: string[];
}

const taskStates = new Set(['Pending', 'Claimed', 'Review', 'Verified', 'Failed']);
const MAX_RESPONSE_BYTES = 1_048_576;

async function readJson(response: Response): Promise<unknown> {
  if (!response.body) throw new Error('Empty registrar response');
  const reader = response.body.getReader();
  const decoder = new TextDecoder('utf-8', { fatal: true });
  let size = 0;
  let body = '';
  try {
    while (true) {
      const chunk = await reader.read();
      if (chunk.done) break;
      size += chunk.value.byteLength;
      if (size > MAX_RESPONSE_BYTES) throw new Error('Registrar response exceeds 1 MiB');
      body += decoder.decode(chunk.value, { stream: true });
    }
    body += decoder.decode();
    return JSON.parse(body) as unknown;
  } finally {
    await reader.cancel();
    reader.releaseLock();
  }
}

function record(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid registrar data');
  return value as Record<string, unknown>;
}

function text(value: unknown): string {
  if (typeof value !== 'string' || value.length > 262_144) throw new Error('Invalid registrar text');
  return value;
}

function count(value: unknown): number {
  if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) throw new Error('Invalid registrar count');
  return value;
}

export function parseSnapshot(statusInput: unknown, tasksInput: unknown, logsInput: unknown): DashboardSnapshot {
  const input = record(statusInput);
  const status: RegistrarStatus = {
    checkout: text(input.checkout), model: text(input.model), endpoint: text(input.endpoint),
    total_tasks: count(input.total_tasks), pending: count(input.pending), review: count(input.review),
    verified: count(input.verified), failed: count(input.failed),
  };
  if (!Array.isArray(tasksInput) || tasksInput.length > 4096) throw new Error('Invalid registrar tasks');
  const ids = new Set<string>();
  const tasks = tasksInput.map(value => {
    const task = record(value);
    const id = text(task.id);
    const state = text(task.status);
    if (!id || ids.has(id) || !taskStates.has(state)) throw new Error('Invalid registrar task state or ID');
    ids.add(id);
    return {
      id, crate_name: text(task.crate_name), task: text(task.task),
      target_file: text(task.target_file), instruction: text(task.instruction),
      status: state as TaskItem['status'],
    };
  });
  const log = text(record(logsInput).log);
  return { status, tasks, logs: log.split('\n').map(line => line.trim()).filter(Boolean).slice(-30).reverse() };
}

export async function loadSnapshot(origin: string, signal: AbortSignal, fetcher: typeof fetch = fetch): Promise<DashboardSnapshot> {
  const payloads = await Promise.all(['/api/status', '/api/tasks', '/api/logs'].map(async path => {
    const response = await fetcher(`${origin}${path}`, { signal, cache: 'no-store' });
    if (!response.ok) throw new Error(`Registrar request failed (${response.status})`);
    return readJson(response);
  }));
  return parseSnapshot(payloads[0], payloads[1], payloads[2]);
}

export function createRefreshController(
  load: (signal: AbortSignal) => Promise<DashboardSnapshot>,
  publish: (snapshot: DashboardSnapshot) => void,
  fail: (error: unknown) => void,
  busy: (value: boolean) => void,
  timeoutMs = 4_000,
) {
  let active: AbortController | null = null;
  let stopped = false;
  return {
    async refresh() {
      if (stopped || active) return;
      const controller = new AbortController();
      active = controller;
      busy(true);
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      try {
        const snapshot = await load(controller.signal);
        if (controller.signal.aborted) throw new Error('Registrar request timed out');
        if (!stopped) publish(snapshot);
      } catch (error) {
        if (!stopped) fail(error);
      } finally {
        clearTimeout(timer);
        controller.abort();
        active = null;
        if (!stopped) busy(false);
      }
    },
    stop() {
      stopped = true;
      active?.abort();
    },
  };
}
