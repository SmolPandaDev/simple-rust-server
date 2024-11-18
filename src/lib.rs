use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

// This is a type alias for a trait object that holds the type of closure that execute receives. 
// Type aliases makes it easier to re-use long types
type Job = Box<dyn FnOnce() + Send + 'static>;


pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,

}

impl ThreadPool {
    /// Create a new ThreadPool.
    ///
    /// The size is the number of threads in the pool.
    ///
    /// # Panics
    ///
    /// The `new` function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        // the channel implementation that Rust provides is multiple producer, single consumer
        let (sender, receiver) = mpsc::channel();
       
        // to share ownership across multiple threads and allow the threads to mutate the value, we need to use Arc<Mutex<T>>
        // The Arc type will let multiple workers own the receiver
        // Mutex will ensure that only one worker gets a job from the receiver at a time.
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            // For each new worker, we clone the Arc to bump the reference count so the workers can share ownership of the receiver.
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender) }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        // send the job down the sending end of the channel for workers to pick up
        // we have to call unwrap() because if we stopped the receiving threads then send could fail
        // our threads will continue executing as long as thread pool exists so this is safe
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
    
}

// When the pool is dropped we want all the threads to finish their work
impl Drop for ThreadPool {
    fn drop(&mut self) {

        // We have to drop the sender to close the channel otherwise our threads will loop forever searching for jobs
        drop(self.sender.take());

        // we use &mut here because self is a mutable reference and we need to mutate the worker.
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            // the `take` method on Option takes the Some variant out and leaves None in its place
            if let Some(thread) = worker.thread.take() {
                // join takes ownership of it's argument
                thread.join().unwrap();
            }

        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            // we call lock() to acquire the mutex
            // then call unwrap() to panic on errors. Note this may fail if mutex was acquired in a poisoned state
            // which can happen if another thread panicked whilst holding the lock rather than releasing the lock.

            // We unwrap() after recv() to panic if the sender closed down and thus we couldn't receive the job.
            let message = receiver.lock().unwrap().recv();
            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");

                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });

        Worker { id, thread: Some(thread) }
    }
}