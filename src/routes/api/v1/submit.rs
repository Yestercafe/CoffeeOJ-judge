use std::fs;

use actix_web::{web, HttpResponse};
use once_cell::sync::Lazy;

use crate::{models::{self, CODE_EXT}, runner::Runner, file::{TestcaseFile, get_pairwise_testcase_files}};

static RUNNER: Lazy<Runner> = Lazy::new(Default::default);

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
    let ret = format!("{:?}", RUNNER.execute(&source_path, lang, &pairwise_testcase_files));

    fs::remove_file(source_path).unwrap();

    HttpResponse::Ok().body(ret)
}