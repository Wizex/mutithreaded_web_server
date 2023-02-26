use std::sync::{mpsc, Arc, Mutex};
use std::thread;

/// The errors that can be returned by the `ThreadPool`.
pub enum PoolError {
    CreationError(&'static str),
}

type Job = Box<dyn FnOnce() + Send + 'static>;

/// The thread pool.
/// 
/// # Description
/// 
/// Allows you to execute tasks concurrently by maintaining a pool of threads
/// and executing passed tasks.
/// 
/// To create an instance of the `ThreadPool` you can use either the `new` or the `build` function. 
/// The 'execute' method takes a task to execute and sends it to the sending-halt of a channel,
/// then an arbitrary thread receives from the receiving-halt of the channel the task and executes it.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Constructs an instance of the `ThreadPool`.
    ///
    /// The size is number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        Self::init(size)
    }

    /// Builds an instance of the `ThreadPool`.
    ///
    /// The auxuillary function for creating a thread pool.
    ///
    /// The size is number of threads in the pool.
    /// 
    /// Returns `Err` if occured an error, otherwise returns `Ok`.
    pub fn build(size: usize) -> Result<Self, PoolError> {
        if size == 0 {
            return Err(PoolError::CreationError("Number of threads equals zero"));
        }

        Ok(Self::init(size))
    }

    fn init(size: usize) -> Self {
        let mut workers: Vec<_> = Vec::with_capacity(size);

        let (sender, receiver) = mpsc::channel();
        let shared_receiver = Arc::new(Mutex::new(receiver));

        for i in 0..size {
            workers.push(Worker::new(i, Arc::clone(&shared_receiver)));
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&mut self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender.as_mut().unwrap().send(Box::new(f)).unwrap();
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
    id: usize,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv();

            match job {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    job();
                }
                Err(_) => {
                    println!("Worker {id} is disconnected. shutting down.");
                    break;
                }
            }           
        });

        Self { thread: Some(thread), id }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod thread_pool {
        use std::time::Duration;

        use super::*;

        #[test]
        #[should_panic]
        fn wrong_size_new_function() {
            ThreadPool::new(0);
        }
    
        #[test]
        fn wrong_size_build_function() {
            assert!(matches!(ThreadPool::build(0), Err(PoolError::CreationError("Number of threads equals zero"))));
        }

        #[test]
        fn init() {
            let thread_pool = ThreadPool::init(3);

            assert_eq!(thread_pool.workers.capacity(), 3);
            
            assert!(matches!(thread_pool.sender, Some(_)));
        }

        #[test]
        fn execute() {
            let mut thread_pool = ThreadPool::new(1);

            let check = Arc::new(Mutex::new(false));
            let check_clone = Arc::clone(&check);

            thread_pool.execute(move || {
                let mut r = check_clone.lock().unwrap();
                *r = true;
            });

            thread::sleep(Duration::from_secs(1));

            assert_eq!(*check.lock().unwrap(), true);
        }
    }
    
    mod worker {
        use super::*;

        #[test]
        fn new() {
            let (_, receiver) = mpsc::channel();
            let receiver = Arc::new(Mutex::new(receiver)); 

            let worker = Worker::new(10, receiver);
            
            assert_eq!(worker.id, 10);
            assert!(matches!(worker.thread, Some(_)));
        }
    }
}
