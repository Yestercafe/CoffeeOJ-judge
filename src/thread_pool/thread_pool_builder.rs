use super::ThreadPool;

pub struct ThreadPoolBuilder {
    pool_size: Option<usize>
}

impl ThreadPoolBuilder {
    pub fn new() -> ThreadPoolBuilder {
        ThreadPoolBuilder {
            pool_size: None,
        }
    }

    pub fn set_pool_size(mut self, size: usize) -> ThreadPoolBuilder {
        assert!(size > 0);
        self.pool_size = Some(size);
        self
    }

    pub fn build(self) -> ThreadPool {
        let pool_size = self.pool_size.unwrap_or(num_cpus::get());

        let thread_pool = ThreadPool::new(pool_size);

        for i in 0..pool_size {
            ThreadPool::spawn_thread(i, thread_pool.shared_data.clone());
        }

        thread_pool
    }
}
