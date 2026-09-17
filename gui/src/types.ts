export interface TaskItem {
  id: string;
  crate_name: string;
  task: string;
  target_file: string;
  instruction: string;
  status: 'Pending' | 'Claimed' | 'Review' | 'Verified' | 'Failed';
}

export interface RegistrarStatus {
  checkout: string;
  model: string;
  endpoint: string;
  total_tasks: number;
  pending: number;
  review: number;
  verified: number;
  failed: number;
}

export interface GateStatus {
  id: number;
  name: string;
  command: string;
  status: 'PASS' | 'FAIL' | 'PENDING';
}
