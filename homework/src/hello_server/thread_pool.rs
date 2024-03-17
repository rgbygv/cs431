//! Thread pool that joins all thread when dropped.

// NOTE: Crossbeam channels are MPMC, which means that you don't need to wrap the receiver in
// Arc<Mutex<..>>. Just clone the receiver and give it to each worker thread.
use crossbeam_channel::{unbounded, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Job(Box<dyn FnOnce() + Send + 'static>);

#[derive(Debug)]
struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for Worker {
    /// When dropped, the thread's `JoinHandle` must be `join`ed.  If the worker panics, then this
    /// function should panic too.
    ///
    /// NOTE: The thread is detached if not `join`ed explicitly.
    fn drop(&mut self) {
        println!("[worker {}] is terminating", self._id);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
            println!("[worker {}] joined", self._id)
        }
    }
}

/// Internal data structure for tracking the current job status. This is shared by worker closures
/// via `Arc` so that the workers can report to the pool that it started/finished a job.
#[derive(Debug, Default)]
struct ThreadPoolInner {
    job_count: Mutex<usize>,
    empty_condvar: Condvar,
}

impl ThreadPoolInner {
    /// Increment the job count.
    fn start_job(&self) {
        let mut cnt = self.job_count.lock().unwrap();
        *cnt += 1;
        println!("[tpool] add (job count: {})", *cnt);
    }

    /// Decrement the job count.
    fn finish_job(&self) {
        let mut cnt = self.job_count.lock().unwrap();
        *cnt -= 1;
        println!("[tpool] finish (job count: {})", *cnt);
    }

    /// Wait until the job count becomes 0.
    ///
    /// NOTE: We can optimize this function by adding another field to `ThreadPoolInner`, but let's
    /// not care about that in this homework.
    fn wait_empty(&self) {
        let cvar = &self.empty_condvar;
        let mut cnt = self.job_count.lock().unwrap();
        while !*cnt == 0 {
            cnt = cvar.wait(cnt).unwrap();
        }
    }
}

/// Thread pool.
#[derive(Debug)]
pub struct ThreadPool {
    _workers: Vec<Worker>,
    job_sender: Option<Sender<Job>>,
    pool_inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    /// Create a new ThreadPool with `size` threads.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);
        let (job_sender, job_receiver) = unbounded::<Job>();
        let pool_inner = Arc::new(ThreadPoolInner {
            job_count: Mutex::new(0),
            empty_condvar: Condvar::new(),
        });
        let mut _workers = vec![];
        for i in 0..size {
            let job_receiver = job_receiver.clone();
            let pool_inner_clone = Arc::clone(&pool_inner);
            let thread = thread::spawn(move || loop {
                let job = job_receiver.recv();
                match job {
                    Ok(job) => {
                        pool_inner_clone.start_job();
                        println!("[worker {}] starts a job", i);
                        (job.0)();
                        pool_inner_clone.finish_job();
                        println!("[worker {}] finishes a job", i);
                    }
                    Err(crossbeam_channel::RecvError) => {
                        // This will happen if all `ThreadPool` clones are dropped.
                        break;
                    }
                }
            });
            _workers.push(Worker {
                _id: i,
                thread: Some(thread),
            });
        }
        Self {
            _workers,
            pool_inner,
            job_sender: Some(job_sender),
        }
    }

    /// Execute a new job in the thread pool.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        if let Some(sender) = &self.job_sender {
            sender.send(Job(job)).expect("Failed to send job to worker")
        }
    }

    /// Block the current thread until all jobs in the pool have been executed.
    ///
    /// NOTE: This method has nothing to do with `JoinHandle::join`.
    pub fn join(&self) {
        self.pool_inner.wait_empty()
    }
}

impl Drop for ThreadPool {
    /// When dropped, all worker threads' `JoinHandle` must be `join`ed. If the thread panicked,
    /// then this function should panic too.
    fn drop(&mut self) {
        self.job_sender.take();
    }
}
