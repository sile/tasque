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
#![warn(missing_docs)]
extern crate num_cpus;
extern crate prometrics;

pub use queue::{TaskQueue, TaskQueueBuilder};

pub mod metrics;

mod queue;
mod task;
mod worker;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn single_worker_works() {
        let (tx, rx) = mpsc::channel();
        let queue = TaskQueueBuilder::new().worker_count(1).finish();
        for (i, tx) in (0..3).map(|i| (i, tx.clone())) {
            queue.enqueue(move || {
                thread::sleep(Duration::from_millis(90 - i * 30));
                let _ = tx.send(i);
            });
        }
        assert_eq!(queue.len(), 3);
        assert_eq!(rx.recv().ok(), Some(0));

        assert_eq!(queue.len(), 2);
        assert_eq!(rx.recv().ok(), Some(1));

        assert_eq!(queue.len(), 1);
        assert_eq!(rx.recv().ok(), Some(2));

        assert_eq!(queue.len(), 0);
        assert_eq!(queue.worker_count(), 1);
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
        assert_eq!(queue.len(), 3);
        assert_eq!(rx.recv().ok(), Some(2));
        assert_eq!(rx.recv().ok(), Some(1));
        assert_eq!(rx.recv().ok(), Some(0));
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.worker_count(), 3);
    }

    #[test]
    fn worker_restart_works() {
        let (tx, rx) = mpsc::channel();
        let queue = TaskQueueBuilder::new().worker_count(1).finish();
        queue.enqueue(|| panic!());
        thread::sleep(Duration::from_millis(1));
        queue.enqueue(move || tx.send(0).unwrap());
        assert_eq!(rx.recv().ok(), Some(0));
        assert_eq!(queue.len(), 0);
    }
}
