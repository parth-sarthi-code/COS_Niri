use std::panic::AssertUnwindSafe;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;

type Task = Box<dyn FnOnce() + Send + 'static>;

pub struct TaskWorker {
    sender: Mutex<Sender<Task>>,
}

static WORKER: OnceLock<TaskWorker> = OnceLock::new();

impl TaskWorker {
    fn spawn_worker_thread() -> Sender<Task> {
        let (sender, receiver) = channel::<Task>();

        thread::Builder::new()
            .name("cos-task-worker".to_string())
            .spawn(move || {
                while let Ok(task) = receiver.recv() {
                    let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
                        task();
                    }));
                }
            })
            .expect("Failed to spawn cos-task-worker thread");

        sender
    }

    fn global() -> &'static TaskWorker {
        WORKER.get_or_init(|| TaskWorker {
            sender: Mutex::new(Self::spawn_worker_thread()),
        })
    }

    /// Dispatch a background task to the reusable worker thread (hardened against task panics & thread death)
    pub fn dispatch<F>(f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let worker = Self::global();
        let task = Box::new(f);

        let mut guard = worker.sender.lock().unwrap_or_else(|e| e.into_inner());
        if guard.send(task).is_err() {
            // Worker thread disconnected; respawn worker thread and retry dispatch
            *guard = Self::spawn_worker_thread();
        }
    }
}
