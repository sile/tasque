use std::sync::Arc;
use std::sync::mpsc::{self, SendError};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use metrics::Metrics;
use task::Task;

#[derive(Debug)]
pub struct Worker {
    task_rx: mpsc::Receiver<Task>,
    round: Arc<AtomicUsize>,
    metrics: Arc<Metrics>,
}
impl Worker {
    pub fn start(metrics: Arc<Metrics>) -> WorkerHandle {
        let round = Arc::new(AtomicUsize::new(0));
        let (task_tx, task_rx) = mpsc::channel();
        let mut worker = Worker {
            task_rx,
            round: Arc::clone(&round),
            metrics,
        };
        thread::spawn(move || while worker.run_once() {});
        WorkerHandle {
            task_tx,
            round,
            next_round: 0,
        }
    }
    fn run_once(&mut self) -> bool {
        if let Ok(task) = self.task_rx.recv() {
            self.metrics.task_duration_seconds.time(|| task.execute());
            self.round.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            false
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.round.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Debug)]
pub struct WorkerHandle {
    task_tx: mpsc::Sender<Task>,
    round: Arc<AtomicUsize>,
    next_round: usize,
}
impl WorkerHandle {
    pub fn try_execute(&mut self, task: Task) -> Result<Option<Task>, Task> {
        if self.round.load(Ordering::SeqCst) == self.next_round {
            if let Err(SendError(task)) = self.task_tx.send(task) {
                Err(task)
            } else {
                self.next_round += 1;
                Ok(None)
            }
        } else {
            Ok(Some(task))
        }
    }
}
