use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use prometrics::metrics::MetricBuilder;
use num_cpus;

use metrics::Metrics;
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
    metrics: MetricBuilder,
}
impl TaskQueueBuilder {
    /// Makes a new `TaskQueueBuilder` instance.
    pub fn new() -> Self {
        TaskQueueBuilder {
            worker_count: num_cpus::get(),
            metrics: MetricBuilder::new(),
        }
    }

    /// Sets the number of worker threads which the queue to be built will spawn.
    pub fn worker_count(&mut self, count: usize) -> &mut Self {
        self.worker_count = count;
        self
    }

    /// Updates the settings of metrics using the given function.
    pub fn metrics<F>(&mut self, f: F) -> &mut Self
    where
        F: FnOnce(&mut MetricBuilder),
    {
        f(&mut self.metrics);
        self
    }

    /// Builds a `TaskQueue` instance.
    pub fn finish(&self) -> TaskQueue {
        let (task_tx, task_rx) = mpsc::channel();
        let metrics = Arc::new(Metrics::new(self.metrics.clone()));
        metrics.workers.set(self.worker_count as f64);
        let workers = (0..self.worker_count)
            .map(|_| Worker::start(Arc::clone(&metrics)))
            .collect();
        let mut manager = TaskQueueManager {
            task_rx,
            workers,
            seq_num: 0,
            tasks: VecDeque::new(),
            metrics: Arc::clone(&metrics),
        };
        thread::spawn(move || while manager.run_once() {});
        TaskQueue { task_tx, metrics }
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
    metrics: Arc<Metrics>,
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
        self.metrics.enqueued_tasks.increment();
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
    tasks: VecDeque<Task>,
    metrics: Arc<Metrics>,
}
impl TaskQueueManager {
    fn run_once(&mut self) -> bool {
        while let Ok(task) = self.task_rx.try_recv() {
            self.tasks.push_back(task);
        }
        if self.tasks.is_empty() {
            if let Ok(task) = self.task_rx.recv() {
                self.tasks.push_back(task);
            } else {
                return false;
            }
        }
        if let Some(task) = self.tasks.pop_front() {
            if let Some(task) = self.dispatch(task) {
                self.tasks.push_front(task);
                thread::sleep(Duration::from_millis(1));
            } else {
                self.metrics.dequeued_tasks.increment();
            }
        }
        true
    }
    fn dispatch(&mut self, mut task: Task) -> Option<Task> {
        for _ in 0..self.workers.len() {
            self.seq_num = self.seq_num.wrapping_add(1);
            let i = self.seq_num % self.workers.len();
            match self.workers[i].try_execute(task) {
                Err(t) => {
                    self.workers[i] = Worker::start(Arc::clone(&self.metrics));
                    self.metrics.worker_restarts.increment();
                    task = t;
                }
                Ok(Some(t)) => {
                    task = t;
                }
                Ok(None) => return None,
            }
        }
        Some(task)
    }
}
