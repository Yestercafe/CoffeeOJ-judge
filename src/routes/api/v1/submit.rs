use std::fs;

use actix_web::{http::StatusCode, rt::Runtime, web, HttpResponse};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{
    file::{get_pairwise_testcase_files, TestcaseFile},
    judge::JudgeErr,
    models::{self, SubmissionStatus, CODE_EXT},
    runner::Runner,
};

static RUNNER: Lazy<Runner> = Lazy::new(Default::default);

type SubmissionStatusCode = i16;
#[derive(Serialize, Deserialize)]
struct SubmitRet {
    status: SubmissionStatusCode,
    info: String,
}

#[tracing::instrument(
    name = "Submit code",
    skip(form),
    fields(
        source = %form.source,
        lang = %form.lang,
    )
)]
pub async fn submit(form: web::Json<models::Submission>) -> HttpResponse {
    // initialize static objects
    Lazy::force(&crate::models::CODE_EXT);
    Lazy::force(&RUNNER);

    // create the source code file
    let source = &form.source;
    let lang = &form.lang;
    let ext = match CODE_EXT.get(lang.as_str()) {
        Some(ext) => *ext,
        None => return HttpResponse::BadRequest().finish(),
    };
    let source_path = format!("Main{}", ext);
    fs::write(source_path.as_str(), source.as_str()).unwrap();

    // exec runner
    let read_path = format!("assets/{}", form.problem_id);
    let lst_read_dir = fs::read_dir(read_path.as_str());
    let mut testcase_files: Vec<TestcaseFile> = vec![];
    if let Ok(lst_read_dir) = lst_read_dir {
        for dir in lst_read_dir {
            let path = format!("{}", dir.unwrap().path().display());
            let sp = path.split_at(read_path.len() + 1);
            testcase_files.push(TestcaseFile::new(sp.1, &path));
        }
    }
    let pairwise_testcase_files = get_pairwise_testcase_files(testcase_files);
    let exec_result = RUNNER.execute(&source_path, lang, &pairwise_testcase_files);
    let ret_status = match exec_result {
        Ok(()) => SubmissionStatus::Accepted,
        Err(JudgeErr::WrongAnswer(_, _)) => SubmissionStatus::WrongAnswer,
        Err(JudgeErr::RuntimeError(_)) => SubmissionStatus::RuntimeError,
        Err(JudgeErr::CompilationError(_)) => SubmissionStatus::CompilationError,
        _ => SubmissionStatus::UnknownError,
    } as SubmissionStatusCode;
    let ret = SubmitRet {
        status: ret_status,
        info: format!("{:?}", exec_result),
    };

    fs::remove_file(source_path).unwrap();

    HttpResponse::Ok().json(ret)
}
