//! Metrics exposed for [prometheus].
//!
//! [prometheus]: https://prometheus.io/
use prometrics::metrics::{Counter, Gauge, Histogram, MetricBuilder};

/// Metrics exposed for [prometheus].
///
/// [prometheus]: https://prometheus.io/
#[derive(Debug)]
pub struct Metrics {
    /// Number of enqueued tasks.
    ///
    /// ```text
    /// tasque_enqueued_tasks_total <value>
    /// ```
    pub enqueued_tasks: Counter,

    /// Number of dequeued tasks.
    ///
    /// Those may not have been executed yet.
    ///
    /// ```text
    /// dequeued_tasks_total <value>
    /// ```
    pub dequeued_tasks: Counter,

    /// Number of workers.
    ///
    /// ```text
    /// tasque_workers <value>
    /// ```
    pub workers: Gauge,

    /// Number of worker restarts.
    ///
    /// ```text
    /// tasque_worker_restarts_total <value>
    /// ```
    pub worker_restarts: Counter,

    /// Execution time of tasks.
    ///
    /// ```text
    /// tasque_task_duration_seconds_buckets{le="0.001"} <value>
    /// tasque_task_duration_seconds_buckets{le="0.01"} <value>
    /// tasque_task_duration_seconds_buckets{le="0.1"} <value>
    /// tasque_task_duration_seconds_buckets{le="1"} <value>
    /// tasque_task_duration_seconds_buckets{le="10"} <value>
    /// tasque_task_duration_seconds_buckets{le="100"} <value>
    /// tasque_task_duration_seconds_buckets{le="+Inf"} <value>
    /// tasque_task_duration_seconds_sum <value>
    /// tasque_task_duration_seconds_count <vaule>
    /// ```
    pub task_duration_seconds: Histogram,
}
impl Metrics {
    pub(crate) fn new(mut builder: MetricBuilder) -> Self {
        builder.namespace("tasque");
        Metrics {
            enqueued_tasks: builder
                .counter("enqueued_tasks_total")
                .help("Number of enqueued tasks")
                .finish()
                .expect("Never fails"),
            dequeued_tasks: builder
                .counter("dequeued_tasks_total")
                .help("Number of dequeued tasks (those may not have been executed yet)")
                .finish()
                .expect("Never fails"),
            workers: builder
                .gauge("workers")
                .help("Number of workers")
                .finish()
                .expect("Never fails"),
            worker_restarts: builder
                .counter("worker_restarts_total")
                .help("Number of worker restarts")
                .finish()
                .expect("Never fails"),
            task_duration_seconds: builder
                .histogram("task_duration_seconds")
                .help("Execution time of tasks")
                .bucket(0.001)
                .bucket(0.01)
                .bucket(0.1)
                .bucket(1.0)
                .bucket(10.0)
                .bucket(100.0)
                .finish()
                .expect("Never fails"),
        }
    }
}
