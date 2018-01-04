use std::collections::VecDeque;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use prometrics::Registry;
use prometrics::metrics::Counter;
use num_cpus;

use metrics::MetricsBuilder;
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
    metrics_builder: MetricsBuilder,
}
impl TaskQueueBuilder {
    /// Makes a new `TaskQueueBuilder` instance.
    pub fn new() -> Self {
        TaskQueueBuilder {
            worker_count: num_cpus::get(),
            metrics_builder: MetricsBuilder::new(),
        }
    }

    /// Sets the name of this queue.
    ///
    /// If it is specified, the metrics of this queue have the label `name="${name}"`.
    pub fn queue_name(&mut self, name: &str) {
        self.metrics_builder.name(name);
    }

    /// Sets the number of worker threads which the queue to be built will spawn.
    pub fn worker_count(&mut self, count: usize) -> &mut Self {
        self.worker_count = count;
        self
    }

    /// Sets the registry of the metrics of this queue.
    ///
    /// The default value is `prometrics::default_registry()`.
    pub fn metrics_registry(&mut self, registry: Registry) {
        self.metrics_builder.registry(registry);
    }

    /// Builds a `TaskQueue` instance.
    pub fn finish(&self) -> TaskQueue {
        let (task_tx, task_rx) = mpsc::channel();
        let workers = (0..self.worker_count)
            .map(|i| Worker::start(i, &self.metrics_builder))
            .collect();
        let mut manager = TaskQueueManager {
            task_rx,
            workers,
            seq_num: 0,
            tasks: VecDeque::new(),
            metrics: Metrics::new(&self.metrics_builder),
            metrics_builder: self.metrics_builder.clone(),
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
    tasks: VecDeque<Task>,
    metrics: Metrics,
    metrics_builder: MetricsBuilder,
}
impl TaskQueueManager {
    fn run_once(&mut self) -> bool {
        while let Ok(task) = self.task_rx.try_recv() {
            self.metrics.enqueued_tasks.increment();
            self.tasks.push_back(task);
        }
        if self.tasks.is_empty() {
            if let Ok(task) = self.task_rx.recv() {
                self.metrics.enqueued_tasks.increment();
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
                    self.workers[i] = Worker::start(i, &self.metrics_builder);
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

#[derive(Debug)]
struct Metrics {
    enqueued_tasks: Counter,
    dequeued_tasks: Counter,
    worker_restarts: Counter,
}
impl Metrics {
    pub fn new(builder: &MetricsBuilder) -> Self {
        Metrics {
            enqueued_tasks: builder
                .counter("enqueued_tasks_total")
                .subsystem("queue")
                .help("Number of enqueued tasks")
                .finish()
                .expect("Never fails"),
            dequeued_tasks: builder
                .counter("dequeued_tasks_total")
                .subsystem("queue")
                .help("Number of dequeued tasks (those may not have been executed yet)")
                .finish()
                .expect("Never fails"),
            worker_restarts: builder
                .counter("restarts_total")
                .subsystem("worker")
                .help("Number of worker restarts")
                .finish()
                .expect("Never fails"),
        }
    }
}
