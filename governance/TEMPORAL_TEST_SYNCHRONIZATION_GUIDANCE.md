# Temporal Test Synchronization Guidance

**Status:** Active — enforced by M19 (Antigravity / Gemini 3.8, 2026-09-16)  
**Applies to:** All test code in `crates/`, `core/`, `dev/`

---

## 1. The Problem

Fixed `thread::sleep` / `tokio::time::sleep` calls in tests are a primary source
of test suite flakiness on slow CI runners, virtual machines, and machines under
load. They also make the test suite unnecessarily slow.

Two distinct failure modes:

- **Race condition masking:** A sleep `long enough to guarantee` some async event
  has occurred is only long enough *on this machine, today*. Under load or on a
  slow CI node the event may not have fired before the assertion.
- **Artificial coupling to wall-clock time:** Tests that manipulate filesystem
  mtimes or cache freshness by sleeping until a full second has elapsed break on
  filesystems with coarser-than-expected granularity and waste one-plus seconds
  unconditionally.

---

## 2. Classification — Test Sleeps Found in Aaroneous

The following classification was made during M19 (commit baseline: `07cd2a9`).

### 2.1 Fixed: `crates/omni/src/matrix/sab_matrix.rs` — `stale_generated_matrix_rebuilds_when_manifest_changes`

**Was:** `std::thread::sleep(std::time::Duration::from_millis(1100))` — slept 1.1s to
guarantee filesystem mtime granularity so the cache file's mtime would be less than
the subsequently written manifest's mtime.

**Now:** Uses `std::fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(10))`
to backdate the cache file immediately after writing it. Test runtime: < 5ms instead of > 1100ms.

### 2.2 Fixed: `core/hypervisor/src/bus_test.rs` — `test_synaptic_task_execution`

**Was:** `tokio::time::sleep(Duration::from_millis(100)).await` — pure simulation
placeholder with no assertion dependency.

**Now:** Removed. Both `write_at` calls are sequential; no cross-call ordering guarantee required.

### 2.3 Legitimate — `core/hypervisor/src/graceful_degradation_tests.rs`

Sleeps of 20–60ms wait for `CircuitBreaker::open_duration` (configured at 20–50ms)
to elapse before asserting state transition. Driven by the production type under test. Legitimate.

### 2.4 Legitimate — `core/hypervisor/src/rate_limit.rs` tests

Sleeps of 40–50ms let the token bucket refill. Token bucket logic is time-based by definition. Legitimate.

### 2.5 Legitimate — `core/hypervisor/src/federation/cluster/tests.rs`

Uses `tokio::time::sleep` for auto-save tick intervals and sentinel cycles driven by
`Duration` configs passed into the type under test. Legitimate.

### 2.6 Legitimate — Production pacing loops

`hid_driver/platform.rs` (10µs poll), `lib.rs` (50ms tick), `mcp_service/service.rs`
(50ms / 300ms retry), `resilience.rs` backoff, CPU-metrics 30ms measurement window — all
legitimate production throttle pacing or async retry intervals. Not test code.

---

## 3. Rules

### 3.1 BANNED: fixed sleep for concurrency synchronization

```rust
// BANNED in tests — races on slow machines
tokio::time::sleep(Duration::from_millis(100)).await;
assert_eq!(shared_state.load(Ordering::SeqCst), EXPECTED);
```

**Replacement — join the task:**
```rust
// ok — join the spawned task; it completes or panics, no race
let result = tokio::spawn(async { do_work() }).await.unwrap();
assert_eq!(result, EXPECTED);
```

**Replacement — use a channel or Notify:**
```rust
// ok — block on the signal, not on a timer
let (tx, rx) = tokio::sync::oneshot::channel();
tokio::spawn(async move { do_work(); tx.send(()).unwrap(); });
rx.await.unwrap();
```

**Replacement — bounded poll with timeout (last resort):**
```rust
// ok — fails fast on CI if condition never arrives
let deadline = std::time::Instant::now() + Duration::from_secs(5);
loop {
    if shared_state.load(Ordering::SeqCst) == EXPECTED { break; }
    assert!(std::time::Instant::now() < deadline, "timed out");
    std::thread::sleep(Duration::from_millis(10));
}
```

### 3.2 BANNED: sleeping for filesystem mtime granularity

```rust
// BANNED — flaky on FAT32/btrfs/network drives, wastes 1s+
fs::write(&path, data).unwrap();
std::thread::sleep(Duration::from_millis(1100));
fs::write(&other_path, other_data).unwrap();
```

**Replacement — backdate the earlier file using `std::fs::FileTimes`:**
```rust
// ok — works on any filesystem, runs in microseconds
let cache_path = write_cache_file(&dir).unwrap();
{
    let past = SystemTime::now() - Duration::from_secs(10);
    let times = std::fs::FileTimes::new().set_modified(past);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&cache_path)
        .unwrap()
        .set_times(times)
        .unwrap();
}
write_manifest_file(&dir).unwrap(); // mtime = now > now - 10s
```

`std::fs::FileTimes` is stable since Rust 1.75. No external crate required.
On Windows: `SetFileTime`. On POSIX: `utimensat`.

### 3.3 PERMITTED: sleeping when the production type under test is time-driven

When the unit under test is itself time-driven (circuit breaker cool-down, token
bucket refill, retry backoff), a sleep is acceptable **only if** the sleep duration
is derived from the same config value used to construct the type, with >= 2x margin:

```rust
// ok — sleep is 2x the configured open_duration
let cb = CircuitBreaker::new(CircuitBreakerConfig {
    open_duration: Duration::from_millis(20),
    ..Default::default()
});
std::thread::sleep(Duration::from_millis(40)); // 2x
assert_eq!(cb.state(), CircuitState::HalfOpen);
```

The `open_duration` itself must be < 100ms so the total wait is bounded.

### 3.4 PERMITTED: production pacing and watchdog sleeps

Production code may use sleep for: throttle pacing, rate-limiter backoff,
retry delay, OS perf counter measurement windows. Do not remove them.

---

## 4. Audit Results Summary (M19 baseline: `07cd2a9`)

| File | Line | Type | Classification | Action |
|---|---|---|---|---|
| `crates/omni/src/matrix/sab_matrix.rs` | 389 | `thread::sleep(1100ms)` | **Flake hazard** | **Fixed** — FileTimes backdate |
| `core/hypervisor/src/bus_test.rs` | 20 | `tokio::time::sleep(100ms)` | **Unnecessary** | **Fixed** — removed |
| `core/hypervisor/src/graceful_degradation_tests.rs` | 124, 152 | `thread::sleep(20–60ms)` | Legitimate | No change |
| `core/hypervisor/src/graceful_degradation_tests.rs` | 296 | `tokio::time::sleep(60s)` | Legitimate (abort test) | No change |
| `core/hypervisor/src/rate_limit.rs` | 275, 299, 328 | `thread::sleep(40–50ms)` | Legitimate (token bucket) | No change |
| `core/hypervisor/src/federation/cluster/tests.rs` | multiple | `tokio::time::sleep` | Legitimate (tick-driven) | No change |
| `core/hypervisor/src/capability_broker.rs` | 1000 | `thread::sleep(20ms)` | Legitimate (debounce window) | No change |
| Production pacing loops (various) | — | `sleep` | Legitimate pacing/backoff | No change |

---

*This document is owned by devtools governance. Update the audit table whenever
a new sleep is introduced or eliminated in Aaroneous test code.*
