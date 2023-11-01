use crate::server::models::submission;

use super::{
    compiler,
    file::{self, Testcase},
    judge::{Judge, JudgeErr},
    runner::{self, Runner},
};

struct Task {
    problem_id: u64,
    testcases_path: String,
    lang: String,
    source_code: String,
}

struct SaveRet {
    submission_id: u64,
    source_path: String,
}

impl SaveRet {
    fn new(submission_id: u64, source_path: &str) -> SaveRet {
        SaveRet {
            submission_id,
            source_path: String::from(source_path),
        }
    }
}

impl Task {
    fn new(problem_id: u64, testcases_path: &str, lang: &str, source_code: &str) -> Task {
        Task {
            problem_id,
            testcases_path: String::from(testcases_path),
            lang: String::from(lang),
            source_code: String::from(source_code),
        }
    }

    fn execute(
        self,
        compiler: &compiler::Compiler2,
        runner: &runner::Runner2,
    ) -> Result<(), JudgeErr> {
        // 1. save source code to file
        let save_ret = file::save_source_code(&self.source_code, &self.lang).map_err(|_| {
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
        let testcases: Vec<Testcase> = Vec::new(); // TODO
        let answer = runner
            .execute(&executable_path, &self.lang, &testcases)
            .map_err(|e| match e {
                runner::Error::CommandEmptyError
                | runner::Error::FileSystemError
                | runner::Error::ForkFailed
                | runner::Error::LanguageNotFoundError => {
                    JudgeErr::InternalError(runner::RunnerErr::UnknownErr(format!("{:?}", e)))
                }
            })?;

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
