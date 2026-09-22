use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Priority tier taxonomy for scheduled tasks.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PriorityTier {
    Background = 0,
    Standard = 1,
    Critical = 2,
}

impl PriorityTier {
    /// Returns the numerical allocation weight for this priority tier.
    pub fn weight(&self) -> f32 {
        match self {
            PriorityTier::Critical => 4.0,
            PriorityTier::Standard => 1.0,
            PriorityTier::Background => 0.25,
        }
    }
}

/// Metadata describing a scheduled task and its execution timing.
#[derive(Clone, Debug)]
pub struct TaskMetadata {
    pub task_id: Uuid,
    pub priority_tier: PriorityTier,
    pub weight_multiplier: f32,
    pub attempt_count: u32,
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub next_eligible_at: Instant,
}

impl TaskMetadata {
    /// Creates a new `TaskMetadata` ready for scheduling.
    pub fn new(task_id: Uuid, priority_tier: PriorityTier, base_delay_ms: u64) -> Self {
        Self {
            task_id,
            priority_tier,
            weight_multiplier: 1.0,
            attempt_count: 0,
            max_attempts: 5,
            base_delay_ms,
            next_eligible_at: Instant::now(),
        }
    }
}

impl PartialEq for TaskMetadata {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
            && self.priority_tier == other.priority_tier
            && self.next_eligible_at == other.next_eligible_at
    }
}

impl Eq for TaskMetadata {}

impl PartialOrd for TaskMetadata {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaskMetadata {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Earliest eligibility first; ties broken by higher priority, then task ID
        self.next_eligible_at
            .cmp(&other.next_eligible_at)
            .then_with(|| other.priority_tier.cmp(&self.priority_tier))
            .then_with(|| self.task_id.cmp(&other.task_id))
    }
}

/// Error type when scheduler is saturated and cannot admit a task.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SchedulerBackpressureError {
    #[error("Scheduler capacity exceeded: task dropped under resource pressure")]
    CapacityExceeded,
}

/// Priority-constrained scheduler managing a binary min-heap of tasks with bounded capacity.
pub struct PriorityScheduler {
    queue: BinaryHeap<Reverse<TaskMetadata>>,
    max_capacity: usize,
}

impl Default for PriorityScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl PriorityScheduler {
    /// Creates a new `PriorityScheduler` with default capacity of 1024 tasks.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Creates a new `PriorityScheduler` with a specified maximum capacity.
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            queue: BinaryHeap::with_capacity(max_capacity),
            max_capacity,
        }
    }

    /// Returns the maximum capacity of the scheduler.
    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    /// Returns `true` if the scheduler queue has reached maximum capacity.
    pub fn is_full(&self) -> bool {
        self.queue.len() >= self.max_capacity
    }

    /// Attempts to enqueue a task into the priority heap enforcing backpressure.
    /// If capacity is saturated:
    /// - `Critical` tasks will evict the lowest-priority task (Background or Standard) to admit the critical work.
    /// - Non-critical tasks are shed with `SchedulerBackpressureError::CapacityExceeded`.
    pub fn try_schedule(&mut self, task: TaskMetadata) -> Result<(), SchedulerBackpressureError> {
        if self.queue.len() >= self.max_capacity {
            if task.priority_tier == PriorityTier::Critical {
                let mut tasks: Vec<TaskMetadata> = self.queue.drain().map(|Reverse(t)| t).collect();
                // Find index of lowest priority task to evict (lowest tier first, then furthest next_eligible_at)
                let mut lowest_idx = None;
                let mut lowest_tier = PriorityTier::Critical;

                for (idx, t) in tasks.iter().enumerate() {
                    if t.priority_tier < lowest_tier {
                        lowest_tier = t.priority_tier;
                        lowest_idx = Some(idx);
                    }
                }

                if let Some(idx) = lowest_idx {
                    tasks.swap_remove(idx);
                    tasks.push(task);
                    for t in tasks {
                        self.queue.push(Reverse(t));
                    }
                    return Ok(());
                }

                // All existing tasks are Critical and queue is full
                for t in tasks {
                    self.queue.push(Reverse(t));
                }
                return Err(SchedulerBackpressureError::CapacityExceeded);
            }

            return Err(SchedulerBackpressureError::CapacityExceeded);
        }

        self.queue.push(Reverse(task));
        Ok(())
    }

    /// Enqueues a task into the priority heap, shedding under backpressure.
    pub fn schedule(&mut self, task: TaskMetadata) {
        let _ = self.try_schedule(task);
    }

    /// Returns the number of scheduled tasks in the queue.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns `true` if the queue contains no tasks.
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Polls the highest-priority task that is eligible for execution at `now`.
    pub fn poll_ready(&mut self, now: Instant) -> Option<TaskMetadata> {
        if let Some(top) = self.queue.peek()
            && top.0.next_eligible_at <= now
        {
            return self.queue.pop().map(|Reverse(task)| task);
        }
        None
    }

    /// Calculates dynamic priority-weighted exponential backoff:
    /// Delay = min(max_cap_ms, (base_delay_ms * 2^attempt_count) / (priority_tier.weight() * weight_multiplier))
    pub fn calculate_backoff(&self, task: &TaskMetadata, max_cap_ms: u64) -> Duration {
        let factor = task.priority_tier.weight() * task.weight_multiplier;
        let effective_factor = if factor <= 0.0 { 1.0 } else { factor };

        let exponential = (task.base_delay_ms as f64) * 2.0f64.powi(task.attempt_count as i32);
        let delay_ms = (exponential / (effective_factor as f64)).round() as u64;
        let capped_delay_ms = delay_ms.min(max_cap_ms);

        Duration::from_millis(capped_delay_ms)
    }

    /// Escalates `priority_tier` to `Critical`, resets `attempt_count` to 0,
    /// and immediately re-enqueues the task at the front of the scheduler.
    pub fn on_compilation_failure(&mut self, mut task: TaskMetadata) {
        task.priority_tier = PriorityTier::Critical;
        task.attempt_count = 0;
        task.next_eligible_at = Instant::now();
        self.schedule(task);
    }

    /// Scales down `weight_multiplier` on all background tasks to defer low-priority executions.
    pub fn on_resource_pressure(&mut self, factor: f32) {
        let mut tasks: Vec<TaskMetadata> = self.queue.drain().map(|Reverse(task)| task).collect();
        for task in &mut tasks {
            if task.priority_tier == PriorityTier::Background {
                task.weight_multiplier *= factor;
            }
        }
        for task in tasks {
            self.queue.push(Reverse(task));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        let mut scheduler = PriorityScheduler::new();
        let now = Instant::now();

        let mut background_task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Background, 100);
        background_task.next_eligible_at = now;

        let mut critical_task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Critical, 100);
        critical_task.next_eligible_at = now;

        let mut standard_task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Standard, 100);
        standard_task.next_eligible_at = now;

        // Schedule in non-priority order
        scheduler.schedule(background_task);
        scheduler.schedule(critical_task.clone());
        scheduler.schedule(standard_task);

        // First polled must be Critical
        let first = scheduler.poll_ready(now).expect("Must poll first task");
        assert_eq!(first.priority_tier, PriorityTier::Critical);

        // Second polled must be Standard
        let second = scheduler.poll_ready(now).expect("Must poll second task");
        assert_eq!(second.priority_tier, PriorityTier::Standard);

        // Third polled must be Background
        let third = scheduler.poll_ready(now).expect("Must poll third task");
        assert_eq!(third.priority_tier, PriorityTier::Background);

        // Queue must now be empty
        assert!(scheduler.poll_ready(now).is_none());
    }

    #[test]
    fn test_bounded_capacity_and_preemption() {
        let mut scheduler = PriorityScheduler::with_capacity(2);
        assert_eq!(scheduler.capacity(), 2);
        assert!(!scheduler.is_full());

        let bg1 = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Background, 100);
        let bg2 = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Background, 100);
        let std1 = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Standard, 100);

        assert!(scheduler.try_schedule(bg1).is_ok());
        assert!(scheduler.try_schedule(bg2).is_ok());
        assert!(scheduler.is_full());

        // Standard task should be shed
        assert_eq!(
            scheduler.try_schedule(std1),
            Err(SchedulerBackpressureError::CapacityExceeded)
        );

        // Critical task should evict a background task
        let crit = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Critical, 100);
        assert!(scheduler.try_schedule(crit).is_ok());
        assert_eq!(scheduler.len(), 2);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let scheduler = PriorityScheduler::new();

        let mut task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Standard, 100);
        task.attempt_count = 0;
        assert_eq!(
            scheduler.calculate_backoff(&task, 30_000),
            Duration::from_millis(100)
        );

        task.attempt_count = 1;
        assert_eq!(
            scheduler.calculate_backoff(&task, 30_000),
            Duration::from_millis(200)
        );

        task.attempt_count = 2;
        assert_eq!(
            scheduler.calculate_backoff(&task, 30_000),
            Duration::from_millis(400)
        );

        // Critical task (weight 4.0): backoff interval compressed by 4
        let mut critical_task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Critical, 100);
        critical_task.attempt_count = 2; // (100 * 4) / 4.0 = 100ms
        assert_eq!(
            scheduler.calculate_backoff(&critical_task, 30_000),
            Duration::from_millis(100)
        );

        // Background task (weight 0.25): backoff interval elongated by 4
        let mut bg_task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Background, 100);
        bg_task.attempt_count = 0; // 100 / 0.25 = 400ms
        assert_eq!(
            scheduler.calculate_backoff(&bg_task, 30_000),
            Duration::from_millis(400)
        );

        // Cap verification
        task.attempt_count = 10; // 100 * 1024 = 102,400ms -> capped at 30,000ms
        assert_eq!(
            scheduler.calculate_backoff(&task, 30_000),
            Duration::from_millis(30_000)
        );
    }

    #[test]
    fn test_compilation_failure_escalation() {
        let mut scheduler = PriorityScheduler::new();
        let now = Instant::now();

        let mut task = TaskMetadata::new(Uuid::new_v4(), PriorityTier::Background, 100);
        task.attempt_count = 3;
        task.next_eligible_at = now + Duration::from_secs(60);

        scheduler.on_compilation_failure(task);

        assert_eq!(scheduler.len(), 1);
        let escalated = scheduler
            .poll_ready(Instant::now())
            .expect("Must be immediately eligible");
        assert_eq!(escalated.priority_tier, PriorityTier::Critical);
        assert_eq!(escalated.attempt_count, 0);
    }
}
