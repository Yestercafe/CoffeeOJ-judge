pub mod comparer;
pub mod compiler;
pub mod consts;
pub mod file;
pub mod macros;
pub mod runner;
pub mod task;

#[derive(Debug)]
pub enum JudgeStatus {
    Halt,
    Pending,
    Accepted,
    WrongAnswer(usize, usize),
    CompilationError(String),
    RuntimeError(String),
    TimeLimitExceeded(u64),
    MemoLimitExceeded(u64),
    UnknownError(String),
}
