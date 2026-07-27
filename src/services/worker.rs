use std::sync::mpsc::{channel, Sender};
use std::sync::OnceLock;
use std::thread;

type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct TaskWorker {
    sender: Sender<Task>,
}

static WORKER: OnceLock<TaskWorker> = OnceLock::new();

impl TaskWorker {
    fn global() -> &'static TaskWorker {
        WORKER.get_or_init(|| {
            let (sender, receiver) = channel::<Task>();

            thread::Builder::new()
                .name("cos-task-worker".to_string())
                .spawn(move || {
                    while let Ok(task) = receiver.recv() {
                        task();
                    }
                })
                .expect("Failed to spawn cos-task-worker thread");

            TaskWorker { sender }
        })
    }

    /// Dispatch a background task to the single reusable worker thread
    pub fn dispatch<F>(f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let worker = Self::global();
        let _ = worker.sender.send(Box::new(f));
    }
}
