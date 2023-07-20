use crate::metrics::Metrics;
use crate::task::Task;
use crate::worker::{Worker, WorkerHandle};
use prometrics::metrics::MetricBuilder;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

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
        let (ctrl_tx, ctrl_rx) = mpsc::channel();
        let metrics = Arc::new(Metrics::new(self.metrics.clone()));
        let mut manager = TaskQueueManager {
            task_rx,
            ctrl_rx,
            ctrl_tx: ctrl_tx.clone(),
            generation: 0,
            metrics: Arc::clone(&metrics),
        };
        thread::spawn(move || while manager.run_once() {});
        let queue = TaskQueue {
            task_tx,
            ctrl_tx,
            metrics,
        };
        queue.set_worker_count(self.worker_count);
        queue
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
    ctrl_tx: mpsc::Sender<Control>,
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
        let _ = self.task_tx.send(Task::new(task, self.ctrl_tx.clone()));
        self.metrics.enqueued_tasks.increment();
    }

    /// Updates the number of worker threads of this queue.
    pub fn set_worker_count(&self, count: usize) {
        let ctrl = Control::SetWorkerCount(count);
        let _ = self.ctrl_tx.send(ctrl);
    }

    /// Returns the number of worker threads of this queue.
    pub fn worker_count(&self) -> usize {
        self.metrics.workers.value() as usize
    }

    /// Returns the number of tasks (enqueued but not yet dequeued) in this queue.
    pub fn len(&self) -> usize {
        (self.metrics.enqueued_tasks.value() - self.metrics.dequeued_tasks.value()) as usize
    }

    /// Returns `true` if this queue is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
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
    ctrl_rx: mpsc::Receiver<Control>,
    ctrl_tx: mpsc::Sender<Control>,
    generation: usize,
    metrics: Arc<Metrics>,
}
impl TaskQueueManager {
    fn run_once(&mut self) -> bool {
        match self.ctrl_rx.recv().expect("Never fails") {
            Control::SetWorkerCount(count) => {
                self.generation += 1;
                for _ in 0..count {
                    let worker = Worker::start(self.generation, Arc::clone(&self.metrics));
                    let _ = self.ctrl_tx.send(Control::PutToPool(worker));
                }
                self.metrics.workers.set(count as f64);
            }
            Control::PutToPool(worker) => {
                if worker.generation() != self.generation {
                    return true;
                }
                if let Ok(task) = self.task_rx.recv() {
                    self.metrics.dequeued_tasks.increment();
                    if let Err(task) = worker.try_execute(task) {
                        let worker = Worker::start(self.generation, Arc::clone(&self.metrics));
                        worker.try_execute(task).expect("Never fails");
                        self.metrics.worker_restarts.increment();
                    }
                } else {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug)]
pub enum Control {
    SetWorkerCount(usize),
    PutToPool(WorkerHandle),
}
