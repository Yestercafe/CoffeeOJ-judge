use std::{fs, sync::Arc};

use super::{
    compiler,
    file::{self, get_pairwise_testcase_files, TestcaseFile},
    judge::JudgeStatus,
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
        compiler: Arc<compiler::Compiler>,
        runner: Arc<runner::Runner>,
    ) -> JudgeStatus {
        // 1. save source code to file
        let save_ret = match file::save_source_code(&self.source_code, &self.lang) {
            Ok(s) => s,
            Err(e) => return JudgeStatus::UnknownError(format!("{:?}", e)),
        };

        // 2. compile
        let executable_path = match compiler.compile(&save_ret, &self.lang) {
            Ok(s) => s,
            Err(e) => {
                return match e {
                    compiler::Error::CompilationError(msg) => JudgeStatus::CompilationError(msg),
                    compiler::Error::LanguageNotFoundError
                    | compiler::Error::ForkFailed
                    | compiler::Error::NoCompilationLogError => {
                        JudgeStatus::CompilationError(format!("{:?}", e))
                    }
                }
            }
        };

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

        let answer = match runner.execute(&executable_path, &self.lang, &testcases) {
            Ok(a) => a,
            Err(e) => return JudgeStatus::UnknownError(format!("{:?}", e)),
        };

        // 4. remove compilation intermediate files (runner.clean)

        // TODO defer.1. update database (db.update(id, xxx))
        // do defer in thread pool

        return answer.get_run_status_owned();
    }
}
