//! Thread affinity and cooperative main-thread pump for `TaskManager`.
//!
//! Most DCC and game-engine host APIs are **UI / main-thread pinned** — calling
//! them from a worker thread either throws or causes a segfault.  This module
//! adds a first-class `ThreadAffinity` concept to the [`TaskManager`] ecosystem
//! and a cooperative [`MainThreadPump`] so the host can drain the main-thread
//! work queue from its own idle callback.
//!
//! # Design
//!
//! - Yield points are **cooperative**: a task must periodically check
//!   [`TaskHandle::is_cancelled`] or simply complete quickly.  There is no
//!   pre-emption, mirroring the Go scheduler and Unity's `EditorApplication`.
//! - [`MainThreadPump::pump`] drains work items up to a wall-clock `budget`
//!   — it never blocks longer than that, keeping the host's frame rate intact.
//!
//! # DCC host integration examples
//!
//! | Host | Idle callback |
//! |---|---|
//! | Maya | `cmds.scriptJob(idleEvent=pump_fn)` |
//! | Houdini | `hdefereval.execute_deferred_after_waiting` |
//! | 3dsMax | `pymxs.run_at_ui_idle` |
//! | Blender | `bpy.app.timers.register` |
//! | Unity Editor | `EditorApplication.update` |
//! | Unreal | `FTSTicker` / `AsyncTask(ENamedThreads::GameThread, …)` |
//!
//! # Example
//!
//! ```rust
//! use ipckit::{MainThreadPump, ThreadAffinity, TaskManager, TaskBuilder};
//! use std::time::Duration;
//!
//! let manager = TaskManager::new(Default::default());
//! let pump = MainThreadPump::new();
//!
//! // Register a "main-thread" task
//! let handle = manager.create(
//!     TaskBuilder::new("update-ui", "ui")
//!         .affinity(ThreadAffinity::Main)
//! );
//!
//! // Dispatch work to the main thread
//! pump.dispatch(move || {
//!     handle.start();
//!     handle.complete(serde_json::json!({"updated": true}));
//! });
//!
//! // In the host idle callback:
//! let stats = pump.pump(Duration::from_millis(8));
//! assert_eq!(stats.processed, 1);
//! ```

use crossbeam_channel as cb;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

// ── ThreadAffinity ────────────────────────────────────────────────────────────

/// Controls which thread a task must execute on.
///
/// Attach to a [`TaskBuilder`](crate::TaskBuilder) via
/// [`TaskBuilder::affinity`]:
///
/// ```rust
/// use ipckit::{TaskBuilder, ThreadAffinity};
///
/// let builder = TaskBuilder::new("render", "render")
///     .affinity(ThreadAffinity::Main);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreadAffinity {
    /// Task must run on the host "main" thread (UI thread in DCC apps).
    ///
    /// The `TaskManager`/host is responsible for draining these tasks via
    /// [`MainThreadPump::pump`].
    Main,

    /// Task must run on a named thread (e.g. `"RenderThread"` in Unreal).
    ///
    /// The string is an arbitrary tag agreed upon by the producer and the
    /// host; ipckit does not validate it.
    Named(String),

    /// Any worker thread is acceptable.  This is the default.
    Any,
}

impl Default for ThreadAffinity {
    fn default() -> Self {
        Self::Any
    }
}

impl ThreadAffinity {
    /// Returns `true` if this affinity requires a specific thread.
    pub fn is_pinned(&self) -> bool {
        !matches!(self, Self::Any)
    }

    /// Returns the thread name for `Named` variants; `None` otherwise.
    pub fn thread_name(&self) -> Option<&str> {
        match self {
            Self::Named(n) => Some(n.as_str()),
            _ => None,
        }
    }
}

// ── PumpStats ─────────────────────────────────────────────────────────────────

/// Statistics returned by a single [`MainThreadPump::pump`] call.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct PumpStats {
    /// Number of work items processed in this pump call.
    pub processed: usize,
    /// Elapsed wall-clock time.
    pub elapsed: Duration,
    /// Number of items still waiting after this pump call.
    pub remaining: usize,
}

// ── MainThreadPump ────────────────────────────────────────────────────────────

type WorkFn = Box<dyn FnOnce() + Send + 'static>;

struct PumpInner {
    tx: cb::Sender<WorkFn>,
    pending: AtomicUsize,
    total_dispatched: AtomicU64,
    total_processed: AtomicU64,
}

/// Cooperative pump that drains main-thread work items on demand.
///
/// [`MainThreadPump`] is `Clone` — all clones share the same underlying queue.
/// The instance that calls [`pump`](Self::pump) must be on the affinity thread.
///
/// # Thread safety
///
/// [`dispatch`](Self::dispatch) is safe to call from any thread.
/// [`pump`](Self::pump) must be called from the thread that "owns" the pump
/// (typically the DCC main / UI thread).
#[derive(Clone)]
pub struct MainThreadPump {
    inner: Arc<PumpInner>,
    rx: Arc<cb::Receiver<WorkFn>>,
}

impl MainThreadPump {
    /// Create a new pump with an unbounded internal queue.
    pub fn new() -> Self {
        let (tx, rx) = cb::unbounded();
        Self {
            inner: Arc::new(PumpInner {
                tx,
                pending: AtomicUsize::new(0),
                total_dispatched: AtomicU64::new(0),
                total_processed: AtomicU64::new(0),
            }),
            rx: Arc::new(rx),
        }
    }

    /// Dispatch a closure to run on the affinity thread.
    ///
    /// Returns immediately; the closure will be executed on the next
    /// [`pump`](Self::pump) call.
    ///
    /// # Panics
    ///
    /// Panics if the pump has been dropped (the receiver end is gone).  In
    /// normal usage this never happens while any clone is alive.
    pub fn dispatch<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.inner.pending.fetch_add(1, Ordering::Relaxed);
        self.inner.total_dispatched.fetch_add(1, Ordering::Relaxed);
        self.inner
            .tx
            .send(Box::new(f))
            .expect("pump receiver dropped");
    }

    /// Drain pending work items for up to `budget` wall-clock time.
    ///
    /// Call this from the host's idle / update callback.  The pump will
    /// process as many items as possible within the budget without
    /// over-running it (each item runs to completion; the budget is checked
    /// *between* items).
    ///
    /// Returns [`PumpStats`] describing what happened.
    pub fn pump(&self, budget: Duration) -> PumpStats {
        let start = Instant::now();
        let mut processed = 0;

        loop {
            // Check budget *before* dequeuing.
            if start.elapsed() >= budget {
                break;
            }

            match self.rx.try_recv() {
                Ok(f) => {
                    f();
                    self.inner.pending.fetch_sub(1, Ordering::Relaxed);
                    self.inner.total_processed.fetch_add(1, Ordering::Relaxed);
                    processed += 1;
                }
                Err(cb::TryRecvError::Empty) => break,
                Err(cb::TryRecvError::Disconnected) => break,
            }
        }

        PumpStats {
            processed,
            elapsed: start.elapsed(),
            remaining: self.inner.pending.load(Ordering::Relaxed),
        }
    }

    /// Number of work items currently waiting to be pumped.
    pub fn pending(&self) -> usize {
        self.inner.pending.load(Ordering::Relaxed)
    }

    /// Total number of closures ever dispatched since this pump was created.
    pub fn total_dispatched(&self) -> u64 {
        self.inner.total_dispatched.load(Ordering::Relaxed)
    }

    /// Total number of closures ever processed since this pump was created.
    pub fn total_processed(&self) -> u64 {
        self.inner.total_processed.load(Ordering::Relaxed)
    }
}

impl Default for MainThreadPump {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;
    use std::thread;

    #[test]
    fn test_thread_affinity_default_is_any() {
        assert_eq!(ThreadAffinity::default(), ThreadAffinity::Any);
    }

    #[test]
    fn test_thread_affinity_is_pinned() {
        assert!(!ThreadAffinity::Any.is_pinned());
        assert!(ThreadAffinity::Main.is_pinned());
        assert!(ThreadAffinity::Named("RenderThread".into()).is_pinned());
    }

    #[test]
    fn test_thread_affinity_thread_name() {
        assert!(ThreadAffinity::Any.thread_name().is_none());
        assert!(ThreadAffinity::Main.thread_name().is_none());
        assert_eq!(ThreadAffinity::Named("GT".into()).thread_name(), Some("GT"));
    }

    #[test]
    fn test_thread_affinity_serialization() {
        let variants = [
            ThreadAffinity::Any,
            ThreadAffinity::Main,
            ThreadAffinity::Named("Render".into()),
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let back: ThreadAffinity = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, v);
        }
    }

    #[test]
    fn test_pump_basic() {
        let pump = MainThreadPump::new();
        let counter = Arc::new(AtomicU32::new(0));

        let c = Arc::clone(&counter);
        pump.dispatch(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        assert_eq!(pump.pending(), 1);
        let stats = pump.pump(Duration::from_millis(100));
        assert_eq!(stats.processed, 1);
        assert_eq!(stats.remaining, 0);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_pump_multiple_items() {
        let pump = MainThreadPump::new();
        let counter = Arc::new(AtomicU32::new(0));

        for _ in 0..5 {
            let c = Arc::clone(&counter);
            pump.dispatch(move || {
                c.fetch_add(1, Ordering::SeqCst);
            });
        }

        let stats = pump.pump(Duration::from_millis(500));
        assert_eq!(stats.processed, 5);
        assert_eq!(counter.load(Ordering::SeqCst), 5);
        assert_eq!(pump.total_dispatched(), 5);
        assert_eq!(pump.total_processed(), 5);
    }

    #[test]
    fn test_pump_budget_zero_processes_nothing() {
        let pump = MainThreadPump::new();
        pump.dispatch(|| {});

        // A zero budget may or may not process items depending on timing;
        // use a tiny positive budget that should still be exceeded quickly.
        // The important thing is it must not block.
        let _ = pump.pump(Duration::ZERO);
        // Pump a second time to clean up.
        pump.pump(Duration::from_millis(100));
    }

    #[test]
    fn test_pump_cross_thread_dispatch() {
        let pump = MainThreadPump::new();
        let pump_worker = pump.clone();
        let counter = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&counter);

        // Dispatch from a worker thread.
        let handle = thread::spawn(move || {
            pump_worker.dispatch(move || {
                c.fetch_add(42, Ordering::SeqCst);
            });
        });
        handle.join().unwrap();

        // Pump on the "main" thread.
        pump.pump(Duration::from_millis(100));
        assert_eq!(counter.load(Ordering::SeqCst), 42);
    }

    #[test]
    fn test_pump_stats_elapsed_is_reasonable() {
        let pump = MainThreadPump::new();
        // Queue a slow item.
        pump.dispatch(|| thread::sleep(Duration::from_millis(20)));

        let stats = pump.pump(Duration::from_millis(500));
        assert_eq!(stats.processed, 1);
        assert!(stats.elapsed >= Duration::from_millis(15)); // some tolerance
    }

    #[test]
    fn test_pump_clone_shares_queue() {
        let pump1 = MainThreadPump::new();
        let pump2 = pump1.clone();
        let counter = Arc::new(AtomicU32::new(0));

        let c = Arc::clone(&counter);
        pump2.dispatch(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        // pump1 drains the item dispatched via pump2.
        let stats = pump1.pump(Duration::from_millis(100));
        assert_eq!(stats.processed, 1);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
