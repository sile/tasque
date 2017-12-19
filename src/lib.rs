extern crate num_cpus;

pub use queue::{TaskQueue, TaskQueueBuilder};

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
                thread::sleep(Duration::from_millis(30 - i * 10));
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
