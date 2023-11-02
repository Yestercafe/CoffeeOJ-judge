use std::fs;

use actix_web::{web, HttpResponse};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::judge::{
    judge::JudgeStatus, task::Task, compiler, runner, consts::LANG_EXTENSIONS,
};
use crate::server::models::{self, SubmissionStatus};

type SubmissionStatusCode = i16;
#[derive(Serialize, Deserialize)]
struct SubmitRet {
    status: SubmissionStatusCode,
    info: String,
}

use std::sync::Mutex;

static SINGLETON_COMPILER: Lazy<Mutex<compiler::Compiler>> = Lazy::new(|| Mutex::new(compiler::Compiler::new()));
static SINGLETON_RUNNER: Lazy<Mutex<runner::Runner>> = Lazy::new(|| Mutex::new(runner::Runner::new()));

#[tracing::instrument(
    name = "Submit code",
    skip(form),
    fields(
        source = %form.source,
        lang = %form.lang,
    )
)]
pub async fn submit(form: web::Json<models::Submission>) -> HttpResponse {
    // create the source code file
    let source = &form.source;
    let lang = &form.lang;
    let ext = match LANG_EXTENSIONS.get(lang.as_str()) {
        Some(ext) => ext,
        None => return HttpResponse::BadRequest().finish(),
    };
    let source_path = format!("Main{}", ext);
    fs::write(source_path.as_str(), source.as_str()).unwrap();

    let Ok(problem_id) = form.problem_id.parse::<u64>() else {
        fs::remove_file(source_path).unwrap();
        let body = SubmitRet {
            status: SubmissionStatus::UnknownError as SubmissionStatusCode,
            info: "Wrong problem id".to_string(),
        };
        return HttpResponse::Ok() .json(body);
    };

    let testcase_path = format!("assets/{problem_id}");

    // exec task
    let this_task = Task::new(problem_id, &testcase_path, lang, source);
    let exec_result = this_task
        .execute(&SINGLETON_COMPILER.lock().unwrap(), &SINGLETON_RUNNER.lock().unwrap());
    let answer = match exec_result {
        JudgeStatus::Accepted => SubmissionStatus::Accepted,
        JudgeStatus::CompilationError(_) => SubmissionStatus::CompilationError,
        JudgeStatus::WrongAnswer(_, _) => SubmissionStatus::WrongAnswer,
        JudgeStatus::MemoLimitExceeded(_) => SubmissionStatus::MemoLimitExceeded,
        JudgeStatus::TimeLimitExceeded(_) => SubmissionStatus::TimeLimitExceeded,
        JudgeStatus::RuntimeError(_) => SubmissionStatus::RuntimeError,
        JudgeStatus::UnknownError(_) => SubmissionStatus::UnknownError,
        _ => SubmissionStatus::UnknownError,
    } as SubmissionStatusCode;
    let ret = SubmitRet {
        status: answer,
        info: format!("{:?}", exec_result),
    };

    fs::remove_file(source_path).unwrap();

    HttpResponse::Ok().json(ret)
}
