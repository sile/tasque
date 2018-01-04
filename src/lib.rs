//! A simple thread pool library for Rust.
//!
//! # Examples
//!
//! Basic usage:
//!
//! ```
//! use std::sync::mpsc;
//! use std::thread;
//! use std::time::Duration;
//! use tasque::TaskQueueBuilder;
//!
//! // Creates a task queue.
//! // This queue spawns worker threads for executing tasks.
//! let queue = TaskQueueBuilder::new().worker_count(3).finish();
//!
//! // Executes asynchronous tasks.
//! let (tx, rx) = mpsc::channel();
//! for (i, tx) in (0..3).map(|i| (i, tx.clone())) {
//!     queue.enqueue(move || {
//!         thread::sleep(Duration::from_millis(20 - i * 10));
//!         let _ = tx.send(i);
//!     });
//! }
//!
//! // Waits results.
//! assert_eq!(rx.recv().ok(), Some(2));
//! assert_eq!(rx.recv().ok(), Some(1));
//! assert_eq!(rx.recv().ok(), Some(0));
//! ```
//!
//! This library exposes some [prometheus] metrics:
//!
//! [prometheus]: https://prometheus.io/
//!
//! ```no_run
//! # extern crate tasque;
//! # extern crate prometrics;
//! use std::time::Duration;
//! use prometrics::default_gatherer;
//! use tasque::TaskQueueBuilder;
//!
//! # fn main() {
//! let queue = TaskQueueBuilder::new().worker_count(1).finish();
//! queue.enqueue(|| {});
//! std::thread::sleep(Duration::from_millis(10));
//!
//! let metrics = default_gatherer().lock().unwrap().gather().to_text();
//! assert_eq!(metrics,
//! [
//!  "# HELP tasque_queue_dequeued_tasks_total Number of dequeued tasks",
//!  "# TYPE tasque_queue_dequeued_tasks_total counter",
//!  "tasque_queue_dequeued_tasks_total 1",
//!  "# HELP tasque_queue_enqueued_tasks_total Number of enqueued tasks",
//!  "# TYPE tasque_queue_enqueued_tasks_total counter",
//!  "tasque_queue_enqueued_tasks_total 1",
//!  "# HELP tasque_queue_started_workers_total Number of workers started so far",
//!  "# TYPE tasque_queue_started_workers_total counter",
//!  "tasque_queue_started_workers_total 1",
//!  "# HELP tasque_worker_task_duration_seconds Execution time of tasks",
//!  "# TYPE tasque_worker_task_duration_seconds histogram",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"0.001\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"0.01\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"0.1\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"1\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"10\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"100\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_bucket{le=\"+Inf\",thread=\"ThreadId(1)\"} 1",
//!  "tasque_worker_task_duration_seconds_sum{thread=\"ThreadId(1)\"} 0.000001392",
//!  "tasque_worker_task_duration_seconds_count{thread=\"ThreadId(1)\"} 1"
//! ].iter().map(|s| format!("{}\n", s)).collect::<Vec<_>>().join("")
//! );
//! # }
//! ```
#![warn(missing_docs)]
extern crate num_cpus;
extern crate prometrics;

pub use queue::{TaskQueue, TaskQueueBuilder};

mod metrics;
mod queue;
mod task;
mod worker;

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use super::*;

    #[test]
    fn single_worker_works() {
        let (tx, rx) = mpsc::channel();
        let queue = TaskQueueBuilder::new().worker_count(1).finish();
        for (i, tx) in (0..3).map(|i| (i, tx.clone())) {
            queue.enqueue(move || {
                thread::sleep(Duration::from_millis(30 - i * 10));
                let _ = tx.send(i);
            });
        }
        assert_eq!(rx.recv().ok(), Some(0));
        assert_eq!(rx.recv().ok(), Some(1));
        assert_eq!(rx.recv().ok(), Some(2));
    }

    #[test]
    fn multiple_workers_works() {
        let (tx, rx) = mpsc::channel();
        let queue = TaskQueueBuilder::new().worker_count(3).finish();
        for (i, tx) in (0..3).map(|i| (i, tx.clone())) {
            queue.enqueue(move || {
                thread::sleep(Duration::from_millis(20 - i * 10));
                let _ = tx.send(i);
            });
        }
        assert_eq!(rx.recv().ok(), Some(2));
        assert_eq!(rx.recv().ok(), Some(1));
        assert_eq!(rx.recv().ok(), Some(0));
    }

    #[test]
    fn worker_restart_works() {
        let (tx, rx) = mpsc::channel();
        let queue = TaskQueueBuilder::new().worker_count(1).finish();
        queue.enqueue(|| panic!());
        queue.enqueue(move || tx.send(0).unwrap());
        assert_eq!(rx.recv().ok(), Some(0));
    }
}
