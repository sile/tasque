use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use num_cpus;

use task::Task;
use worker::{Worker, WorkerHandle};

/// `TaskQueue` builder.
///
/// # Examples
///
/// ```
/// use tasque::TaskQueueBuilder;
///
/// let queue = TaskQueueBuilder::new().worker_count(4).finish();
/// queue.enqueue(|| println!("Hello"));
/// queue.enqueue(|| println!("World"));
/// ```
#[derive(Debug)]
pub struct TaskQueueBuilder {
    worker_count: usize,
}
impl TaskQueueBuilder {
    /// Makes a new `TaskQueueBuilder` instance.
    pub fn new() -> Self {
        TaskQueueBuilder {
            worker_count: num_cpus::get(),
        }
    }

    /// Sets the number of worker threads which the queue to be built will spawn.
    pub fn worker_count(&mut self, count: usize) -> &mut Self {
        self.worker_count = count;
        self
    }

    /// Builds a `TaskQueue` instance.
    pub fn finish(&self) -> TaskQueue {
        let (task_tx, task_rx) = mpsc::channel();
        let workers = (0..self.worker_count).map(|_| Worker::start()).collect();
        let mut manager = TaskQueueManager {
            task_rx,
            workers,
            seq_num: 0,
            task: None,
        };
        thread::spawn(move || while manager.run_once() {});
        TaskQueue { task_tx }
    }
}
impl Default for TaskQueueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Task queue.
///
/// This queue spawns worker threads for executing registered tasks.
///
/// # Examples
///
/// ```
/// use tasque::TaskQueue;
///
/// let queue = TaskQueue::new();
/// queue.enqueue(|| println!("Hello"));
/// queue.enqueue(|| println!("World"));
/// ```
#[derive(Debug, Clone)]
pub struct TaskQueue {
    task_tx: mpsc::Sender<Task>,
}
impl TaskQueue {
    /// Makes a new `TaskQueue` instance.
    ///
    /// This is equivalent to `TaskQueueBuilder::new().finish()`.
    pub fn new() -> Self {
        TaskQueueBuilder::new().finish()
    }

    /// Enqueues a task.
    ///
    /// The task will be executed by a worker thread.
    ///
    /// If the thread panics while executing the task, it will be automatically restarted.
    /// Note that the task will not be retried in such case.
    pub fn enqueue<F>(&self, task: F)
    where
        F: FnOnce() + Send + 'static,
    {
        assert!(self.task_tx.send(Task::new(task)).is_ok());
    }
}
impl Default for TaskQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
struct TaskQueueManager {
    task_rx: mpsc::Receiver<Task>,
    workers: Vec<WorkerHandle>,
    seq_num: usize,
    task: Option<Task>,
}
impl TaskQueueManager {
    pub fn run_once(&mut self) -> bool {
        if self.task.is_none() {
            if let Ok(task) = self.task_rx.recv() {
                self.task = Some(task);
            } else {
                return false;
            }
        }
        if let Some(task) = self.task.take() {
            self.dispatch(task);
        }
        true
    }
    fn dispatch(&mut self, mut task: Task) {
        let last = self.seq_num % self.workers.len();
        loop {
            self.seq_num += 1;
            let i = self.seq_num % self.workers.len();
            match self.workers[i].try_execute(task) {
                Err(t) => {
                    self.workers[i] = Worker::start();
                    task = t;
                }
                Ok(Some(t)) => {
                    task = t;
                }
                Ok(None) => break,
            }
            if last == i {
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
