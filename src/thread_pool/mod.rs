pub mod tests;
pub mod thread_pool_builder;

use std::{
    fmt::Debug,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{channel, Receiver, Sender},
        Arc, Condvar, Mutex,
    },
    thread,
};

use crate::judge::{
    compiler,
    runner::{self, RunnerJob},
    task::Task,
};

type Thunk<'a> = Box<dyn FnOnce() -> Vec<RunnerJob> + Send + 'a>;

struct Sentinel<'a> {
    id: usize,
    shared_data: &'a Arc<SharedData>,
    active: bool,
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
            self.shared_data
                .active_thread_count
                .fetch_sub(1, Ordering::SeqCst);
            if thread::panicking() {
                self.shared_data
                    .panic_thread_count
                    .fetch_add(1, Ordering::SeqCst);
            }
            ThreadPool::spawn_thread(self.id, self.shared_data.clone());
            self.shared_data.notify_when_idle();
        }
    }
}

pub struct SharedData {
    pub job_sender: Arc<Sender<Thunk<'static>>>,
    pub job_receiver: Mutex<Receiver<Thunk<'static>>>,

    pub global_compiler: Arc<compiler::Compiler>,
    pub global_runner: Arc<runner::Runner>,

    pub empty_trigger: Mutex<()>,
    pub empty_condvar: Condvar,
    pub join_times: AtomicUsize,

    pub max_thread_count: AtomicUsize,
    pub active_thread_count: AtomicUsize,
    pub panic_thread_count: AtomicUsize,
    pub queued_job_count: AtomicUsize,

    pub is_all_done: AtomicBool,
    pub is_all_active: AtomicBool,
}

impl SharedData {
    fn is_idle(&self) -> bool {
        self.queued_job_count.load(Ordering::SeqCst) == 0
            && self.active_thread_count.load(Ordering::SeqCst) == 0
    }

    fn notify_when_idle(&self) {
        if self.is_idle() {
            self.empty_condvar.notify_all();
        }
    }
}

pub struct ThreadPool {
    pub job_sender: Arc<Sender<Thunk<'static>>>,
    pub shared_data: Arc<SharedData>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = channel::<Thunk<'static>>();
        let sender = Arc::new(sender);
        let shared_data = Arc::new(SharedData {
            job_sender: sender.clone(),
            job_receiver: Mutex::new(receiver),
            global_compiler: Arc::new(compiler::Compiler::default()),
            global_runner: Arc::new(runner::Runner::default()),
            empty_trigger: Mutex::new(()),
            empty_condvar: Condvar::new(),
            join_times: AtomicUsize::new(0),
            max_thread_count: AtomicUsize::new(size),
            active_thread_count: AtomicUsize::new(0),
            panic_thread_count: AtomicUsize::new(0),
            queued_job_count: AtomicUsize::new(0),
            is_all_done: AtomicBool::new(false),
            is_all_active: AtomicBool::new(false),
        });

        ThreadPool {
            job_sender: sender,
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

    pub fn is_idle(&self) -> bool {
        self.shared_data.is_idle()
    }

    pub fn awake_all(&self) {
        self.shared_data.is_all_active.store(true, Ordering::SeqCst);
    }

    pub fn block_all(&self) {
        self.shared_data
            .is_all_active
            .store(false, Ordering::SeqCst);
    }

    pub fn stop_all(&self, unconditional: bool) {
        if unconditional {
            let lock = self.shared_data.job_receiver.lock().unwrap();
            while lock.recv().is_ok() {}
        }
        self.shared_data.is_all_done.store(true, Ordering::SeqCst);
    }

    pub fn join(&self) {
        if self.is_idle() {
            return;
        }

        let join_times = self.shared_data.join_times.load(Ordering::SeqCst);
        let mut trigger = self.shared_data.empty_trigger.lock().unwrap();

        while join_times == self.shared_data.join_times.load(Ordering::SeqCst) && !self.is_idle() {
            trigger = self.shared_data.empty_condvar.wait(trigger).unwrap();
        }

        let _ = self.shared_data.join_times.compare_exchange(
            join_times,
            join_times + 1,
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }

    pub fn send_job<F>(&self, job: F)
    where
        F: FnOnce() -> Vec<RunnerJob> + Send + 'static,
    {
        self.shared_data
            .queued_job_count
            .fetch_add(1, Ordering::SeqCst);
        self.job_sender.send(Box::new(job)).unwrap();
    }

    pub fn send_task(&self, task: Task) {
        let shared_data = self.shared_data.clone();
        self.send_job(move || {
            let result = task.execute(
                shared_data.global_compiler.clone(),
                shared_data.global_runner.clone(),
            );

            if let Ok(runner_jobs) = result {
                runner_jobs
            } else {
                println!("{:?}", result.unwrap_err());
                vec![]
            }
        });
    }

    pub fn spawn_thread(id: usize, shared_data: Arc<SharedData>) {
        let builder = thread::Builder::new();

        builder
            .spawn(move || {
                let setinel = Sentinel::new(id, &shared_data);

                loop {
                    if shared_data.is_all_done.load(Ordering::SeqCst) {
                        break;
                    }
                    if !shared_data.is_all_active.load(Ordering::SeqCst) {
                        continue;
                    }

                    let msg = {
                        let lock = shared_data
                            .job_receiver
                            .lock()
                            .expect(format!("Worker #{id}: unable to lock job_receiver").as_str());
                        lock.recv()
                    };

                    let job = match msg {
                        Ok(job) => job,
                        Err(_) => continue,
                    };

                    shared_data
                        .active_thread_count
                        .fetch_add(1, Ordering::SeqCst);
                    shared_data.queued_job_count.fetch_sub(1, Ordering::SeqCst);

                    let runner_jobs: Vec<RunnerJob> = job();
                    println!("Worker #{id}: {:#?}", runner_jobs);
                    for job in runner_jobs {
                        shared_data.queued_job_count.fetch_add(1, Ordering::SeqCst);
                        shared_data
                            .job_sender
                            .send(Box::new(|| {
                                job.execute_once().unwrap();
                                vec![]
                            }))
                            .unwrap();
                    }

                    shared_data
                        .active_thread_count
                        .fetch_sub(1, Ordering::SeqCst);

                    shared_data.notify_when_idle();
                }

                setinel.cancel();
            })
            .unwrap();
    }
}

impl Debug for ThreadPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ThreadPool")
            .field("max_thread_count", &self.max_thread_count())
            .field("active_thread_count", &self.active_thread_count())
            .field("panic_thread_count", &self.panic_thread_count())
            .field("queued_job_count", &self.queued_job_count())
            .finish()
    }
}
