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

    fn set_pool_size(mut self, size: usize) -> ThreadPoolBuilder {
        assert!(size > 0);
        self.pool_size = Some(size);
        self
    }

    fn build(self) -> ThreadPool {
        let pool_size = self.pool_size.unwrap_or(num_cpus::get());

        let thread_pool = ThreadPool::new(pool_size);

        for i in 0..pool_size {
            ThreadPool::spawn_thread(i, thread_pool.shared_data.clone());
        }

        thread_pool
    }
}

#[cfg(test)]
mod test {
    use std::{time::Duration, thread::sleep};

    use crate::judge::task::Task;

    use super::ThreadPoolBuilder;

    #[test]
    fn thread_pool_test1() {
        let thread_pool = ThreadPoolBuilder::new().set_pool_size(8).build();
        for _ in 0..20 {
            let task = Task::new(1, "assets/1", "cpp", "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << 2 * a; }");
            thread_pool.send_task(task);
        }
        thread_pool.start_all();

        sleep(Duration::from_secs(3));

        thread_pool.stop_all(true);
    }
}
