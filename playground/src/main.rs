use std::sync::{Arc, Mutex};
use std::thread;

pub trait Draw {
    fn draw(&self);
}

pub struct Screen {
    pub components: Vec<Box<dyn Draw>>,
}

impl Screen {
    pub fn run(&self) {
        for component in self.components.iter() {
            component.draw();
        }
    }
}

fn main() {
    let counter: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
    let mut handles: Vec<thread::JoinHandle<()>> = vec![];

    for _ in 0..10 {
        let ctr: Arc<Mutex<i32>> = counter.clone();
        let handle: thread::JoinHandle<()> = thread::spawn(move || {
            let mut num: std::sync::MutexGuard<i32> = ctr.lock().unwrap();
            *num += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // println!("Result: {}", *counter.lock().unwrap());
}