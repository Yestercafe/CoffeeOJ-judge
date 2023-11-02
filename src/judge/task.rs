use std::fs;

use super::{
    compiler,
    file::{self, get_pairwise_testcase_files, TestcaseFile},
    judge::JudgeErr,
    runner,
};

pub struct Task {
    #[allow(dead_code)]
    problem_id: u64,
    testcases_path: String,
    lang: String,
    source_code: String,
}

impl Task {
    pub fn new(problem_id: u64, testcases_path: &str, lang: &str, source_code: &str) -> Task {
        Task {
            problem_id,
            testcases_path: String::from(testcases_path),
            lang: String::from(lang),
            source_code: String::from(source_code),
        }
    }

    pub fn execute(
        self,
        compiler: &compiler::Compiler,
        runner: &runner::Runner,
    ) -> Result<(), JudgeErr> {
        // 1. save source code to file
        let save_ret = file::save_source_code(&self.source_code, &self.lang).map_err(|_| {
            // TODO
            // update database: FileSystemError
            JudgeErr::WrongAnswer(1, 2)
        })?;

        // 2. compile
        let executable_path = compiler
            .compile(&save_ret, &self.lang)
            .map_err(|e| match e {
                compiler::Error::CompilationError(msg) => JudgeErr::CompilationError(msg),
                compiler::Error::LanguageNotFoundError
                | compiler::Error::ForkFailed
                | compiler::Error::NoCompilationLogError => {
                    JudgeErr::CompilationError(format!("{:?}", e))
                }
            })?;

        // 3. run (runner.execute)
        let lst_read_dir = fs::read_dir(&self.testcases_path);
        let mut testcase_files: Vec<TestcaseFile> = vec![];
        if let Ok(lst_read_dir) = lst_read_dir {
            for dir in lst_read_dir {
                let path = format!("{}", dir.unwrap().path().display());
                let sp = path.split_at(&self.testcases_path.len() + 1);
                testcase_files.push(TestcaseFile::new(sp.1, &path));
            }
        }
        let testcases = get_pairwise_testcase_files(testcase_files);

        let answer = runner
            .execute(&executable_path, &self.lang, &testcases)
            .map_err(|e| JudgeErr::InternalError(e))?;

        // 4. remove compilation intermediate files (runner.clean)

        let ret: Result<(), JudgeErr> = if let runner::RunStatus::AC = answer.get_run_status() {
            Ok(())
        } else {
            Err(match answer.get_run_status() {
                runner::RunStatus::WA(x, y) => JudgeErr::WrongAnswer(x, y),
                runner::RunStatus::TLE => todo!(),
                runner::RunStatus::MLE => todo!(),
                runner::RunStatus::RE => todo!(),
                runner::RunStatus::Unknown => todo!(),
                runner::RunStatus::AC => unreachable!(),
            })
        };

        // JudgeErr should store time used and mem used

        // TODO defer.1. update database (db.update(id, xxx))
        // do defer in thread pool

        return ret;
    }
}
