use coffee_oj_judge::judge;
use coffee_oj_judge::server::{startup::WebApp, utils};
use once_cell::sync::Lazy;

fn init_lazy() {
    Lazy::force(&judge::consts::LANG_EXTENSIONS);
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_lazy();
    utils::telemetry::setup_log("coj_judge", "info", std::io::stdout);
    let web_app = WebApp::new().await?;
    web_app.run().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use coffee_oj_judge::{judge::task, thread_pool::thread_pool_builder::ThreadPoolBuilder};

    use crate::init_lazy;

    #[test]
    fn manual_main_test() {
        init_lazy();

        let thread_pool = ThreadPoolBuilder::new().build();

        let a_task = task::Task::new(
            1,
            "assets/1",
            "cpp",
            "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << a * 2; return 0; }",
        );
        thread_pool.send_task(a_task);

        let a_task = task::Task::new(1, "assets/1", "python", "print(2 * int(input()))");
        thread_pool.send_task(a_task);

        assert_eq!(thread_pool.active_thread_count(), 0);
        assert_eq!(thread_pool.queued_job_count(), 2);

        thread_pool.awake_all();
        thread_pool.join();

        assert_eq!(thread_pool.queued_job_count(), 0);
    }
}
