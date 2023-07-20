use std::sync::mpsc::{self, SendError};
use std::sync::Arc;
use std::thread;

use metrics::Metrics;
use task::Task;

#[derive(Debug)]
pub struct Worker {
    task_rx: mpsc::Receiver<Task>,
    metrics: Arc<Metrics>,
}
impl Worker {
    pub fn start(generation: usize, metrics: Arc<Metrics>) -> WorkerHandle {
        let (task_tx, task_rx) = mpsc::channel();
        let mut worker = Worker { task_rx, metrics };
        thread::spawn(move || while worker.run_once() {});
        WorkerHandle {
            generation,
            task_tx,
        }
    }
    fn run_once(&mut self) -> bool {
        if let Ok(task) = self.task_rx.recv() {
            self.metrics.task_duration_seconds.time(|| task.execute());
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkerHandle {
    generation: usize,
    task_tx: mpsc::Sender<Task>,
}
impl WorkerHandle {
    pub fn generation(&self) -> usize {
        self.generation
    }
    pub fn try_execute(&self, mut task: Task) -> Result<(), Task> {
        task.assign(self.clone());
        if let Err(SendError(task)) = self.task_tx.send(task) {
            Err(task)
        } else {
            Ok(())
        }
    }
}
