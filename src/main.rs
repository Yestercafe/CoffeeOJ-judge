use coffee_oj_judge::judge::{self, compiler, runner, task};
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

fn _main() {
    init_lazy();

    let a_task = task::Task::new(
        1,
        "assets/1",
        "cpp",
        "#include <iostream>\nint main() { int a; std::cin >> a; std::cout << a * 2; return 0; }",
    );
    let a_compiler = compiler::Compiler::new();
    let a_runner = runner::Runner::new();
    let ret = a_task.execute(&a_compiler, &a_runner);
    println!("{:?}", ret);

    let a_task = task::Task::new(1, "assets/1", "python", "print(2 * int(input()))");
    let a_compiler = compiler::Compiler::new();
    let a_runner = runner::Runner::new();
    let ret = a_task.execute(&a_compiler, &a_runner);
    println!("{:?}", ret);
}
