use std::{sync::{Arc, mpsc::{Sender, channel, Receiver}, Mutex, atomic::{AtomicUsize, Ordering, AtomicBool}, Condvar}, thread};

use crate::judge::{task::Task, compiler, runner, judge::JudgeStatus};

type Thunk<'a> = Box<dyn FnOnce() -> JudgeStatus + Send + 'a>;

struct Sentinel<'a> {
    id: usize,
    shared_data: &'a Arc<SharedData>,
    active: bool
}

impl<'a> Sentinel<'a> {
    fn new(id: usize, shared_data: &'a Arc<SharedData>) -> Sentinel<'a> {
        Sentinel {
            id,
            shared_data,
            active: true,
        }
    }

    fn cancel(mut self) {
        self.active = false;
    }
}

impl<'a> Drop for Sentinel<'a> {
    fn drop(&mut self) {
        if self.active {
            self.shared_data.active_thread_count.fetch_sub(1, Ordering::SeqCst);
            if thread::panicking() {
                self.shared_data.panic_thread_count.fetch_add(1, Ordering::SeqCst);
            }
            ThreadPool::spawn_thread(self.id, self.shared_data.clone());
        }
    }
}

pub struct SharedData {
    pub job_receiver: Mutex<Receiver<Thunk<'static>>>,
    pub global_compiler: Arc<compiler::Compiler>,
    pub global_runner: Arc<runner::Runner>,
    pub max_thread_count: AtomicUsize,
    pub active_thread_count: AtomicUsize,
    pub panic_thread_count: AtomicUsize,
    pub queued_job_count: AtomicUsize,
    pub should_stop: AtomicBool,
    pub should_start: AtomicBool,
}

pub struct ThreadPool {
    pub jobs_sender: Sender<Thunk<'static>>,
    pub shared_data: Arc<SharedData>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = channel::<Thunk<'static>>();
        let shared_data = Arc::new(SharedData {
            job_receiver: Mutex::new(receiver),
            global_compiler: Arc::new(compiler::Compiler::new()),
            global_runner: Arc::new(runner::Runner::new()),
            max_thread_count: AtomicUsize::new(size),
            active_thread_count: AtomicUsize::new(0),
            panic_thread_count: AtomicUsize::new(0),
            queued_job_count: AtomicUsize::new(0),
            should_stop: AtomicBool::new(false),
            should_start: AtomicBool::new(false),
        });

        ThreadPool {
            jobs_sender: sender,
            shared_data,
        }
    }

    pub fn max_thread_count(&self) -> usize {
        self.shared_data.max_thread_count.load(Ordering::Relaxed)
    }

    pub fn active_thread_count(&self) -> usize {
        self.shared_data.active_thread_count.load(Ordering::SeqCst)
    }

    pub fn panic_thread_count(&self) -> usize {
        self.shared_data.panic_thread_count.load(Ordering::SeqCst)
    }

    pub fn queued_job_count(&self) -> usize {
        self.shared_data.queued_job_count.load(Ordering::SeqCst)
    }

    pub fn start_all(&self) {
        self.shared_data.should_start.store(true, Ordering::SeqCst);
    }

    pub fn stop_all(&self, unconditional: bool) {
        if unconditional {
            let lock = self.shared_data.job_receiver.lock().unwrap();
            while let Ok(_) = lock.recv() {}
        }
        self.shared_data.should_stop.store(true, Ordering::SeqCst);
    }

    fn send_job<F>(&self, job: F)
    where
        F: FnOnce() -> JudgeStatus + Send + 'static
    {
        self.shared_data.queued_job_count.fetch_add(1, Ordering::SeqCst);
        self.jobs_sender.send(Box::new(job)).unwrap();
    }

    pub fn send_task(&self, task: Task) {
        let shared_data = self.shared_data.clone();
        self.send_job(move || {
            task.execute(shared_data.global_compiler.clone(), shared_data.global_runner.clone())
        });
    }

    pub fn spawn_thread(id: usize, shared_data: Arc<SharedData>) {
        let builder = thread::Builder::new();

        builder.spawn(move || {
            let setinel = Sentinel::new(id, &shared_data);

            loop {
                if shared_data.should_stop.load(Ordering::SeqCst) {
                    break
                }
                if !shared_data.should_start.load(Ordering::SeqCst) {
                    continue
                }

                let msg = {
                    let lock = shared_data.job_receiver.lock().expect(format!("Worker #{id}: unable to lock job_receiver").as_str());
                    lock.recv()
                };

                let job = match msg {
                    Ok(job) => job,
                    Err(_) => continue,
                };

                shared_data.active_thread_count.fetch_add(1, Ordering::SeqCst);
                shared_data.queued_job_count.fetch_sub(1, Ordering::SeqCst);

                let judge_status = job();
                println!("Worker #{id}: {:?}", judge_status);

                shared_data.active_thread_count.fetch_sub(1, Ordering::SeqCst);
            }

            setinel.cancel();
        }).unwrap();
    }
}
