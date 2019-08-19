use std::fmt;
use std::sync::mpsc;

use queue::Control;
use worker::WorkerHandle;

pub struct Task {
    task: Box<dyn FnMut() + Send + 'static>,
    ctrl_tx: mpsc::Sender<Control>,
    worker: Option<WorkerHandle>,
}
impl Task {
    pub fn new<F>(task: F, ctrl_tx: mpsc::Sender<Control>) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let mut f = Some(task);
        Task {
            task: Box::new(move || {
                let f = f.take().expect("Never fails");
                f()
            }),
            ctrl_tx,
            worker: None,
        }
    }
    pub fn assign(&mut self, worker: WorkerHandle) {
        self.worker = Some(worker);
    }
    pub fn execute(mut self) {
        (self.task)();
    }
}
impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Task {{ .. }}")
    }
}
impl Drop for Task {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.ctrl_tx.send(Control::PutToPool(worker));
        }
    }
}
