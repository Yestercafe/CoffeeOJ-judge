use std::collections::BTreeMap;

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static CODE_EXT: Lazy<BTreeMap<&'static str, &'static str>> = Lazy::new(|| {
    BTreeMap::from([
        ("c", ".c"),
        ("cpp", ".cpp"),
        ("rust", ".rs"),
        ("python", ".py"),
    ])
});

#[derive(Clone, Copy)]
pub enum SubmissionStatus {
    Accepted = 0,
    WrongAnswer,
    CompilationError,
    RuntimeError,
    TimeLimitExceeded,
    MemoLimitExceeded,
    UnknownError,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Submission {
    pub source: String,
    pub lang: String,
    pub problem_id: String,
}

// impl Submission {
//     fn new(source: &str, lang: &str, problem_id: &str) -> Self {
//         Self {
//             source: String::from(source),
//             lang: String::from(lang),
//             problem_id: String::from(problem_id),
//         }
//     }
// }
