extern crate prometrics;
extern crate tasque;

use std::thread;
use std::time::Duration;
use tasque::TaskQueue;

fn main() {
    let queue = TaskQueue::new();
    queue.enqueue(|| thread::sleep(Duration::from_millis(1)));
    queue.enqueue(|| thread::sleep(Duration::from_millis(50)));
    queue.enqueue(|| thread::sleep(Duration::from_millis(1000)));
    thread::sleep(Duration::from_millis(100));

    println!(
        "{}",
        prometrics::default_gatherer()
            .lock()
            .unwrap()
            .gather()
            .to_text()
    );
}
