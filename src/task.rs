use std::fmt;

pub struct Task(Box<FnMut() + Send + 'static>);
impl Task {
    pub fn new<F>(task: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        let mut f = Some(task);
        Task(Box::new(move || {
            let f = f.take().expect("Never fails");
            f()
        }))
    }
    pub fn execute(mut self) {
        (self.0)()
    }
}
impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Task(_)")
    }
}
