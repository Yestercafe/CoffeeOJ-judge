#[cfg(test)]
mod test {
    use crate::{
        judge::{judge::JudgeStatus, task::Task},
        thread_pool::thread_pool_builder::ThreadPoolBuilder,
    };

    #[test]
    fn thread_pool_join() {
        let thread_pool = ThreadPoolBuilder::new().build();
        for _ in 0..20 {
            let task = Task::new(
                1,
                "assets/1",
                "cpp",
                "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << 2 * a; }",
            );
            thread_pool.send_task(task);
        }
        thread_pool.awake_all();
        thread_pool.join();

        assert_eq!(thread_pool.active_thread_count(), 0);
        assert_eq!(thread_pool.queued_job_count(), 0);
    }

    #[test]
    fn thread_pool_thread_panic() {
        let thread_pool = ThreadPoolBuilder::new().build();
        for i in 0..20 {
            if i % 4 == 0 {
                thread_pool.send_job(|| -> JudgeStatus { panic!() })
            } else {
                let task = Task::new(
                    1,
                    "assets/1",
                    "cpp",
                    "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << 2 * a; }",
                );
                thread_pool.send_task(task);
            }
        }

        thread_pool.awake_all();
        thread_pool.join();

        assert_eq!(thread_pool.active_thread_count(), 0);
        assert_eq!(thread_pool.queued_job_count(), 0);
        assert_eq!(thread_pool.panic_thread_count(), 5);
    }
}
