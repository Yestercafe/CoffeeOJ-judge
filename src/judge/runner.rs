use std::{collections::BTreeMap, ffi::CString, fs, sync::Mutex};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{self, ForkResult},
};
use toml::{Table, Value};

use crate::{
    c_string,
    judge::{
        consts::CONFIG_PATH,
        file::{Testcase, TestcaseFile},
        judge::Judge,
    },
};

use super::judge::JudgeStatus;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum FileType {
    Stdout,
    Stdin,
    Stderr,
    File(String),
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Error {
    ForkFailed,
    LanguageNotFoundError,
    CommandEmptyError,
    FileSystemError,
}

pub struct Runner {
    running_recipe: Mutex<BTreeMap<String, Option<Vec<String>>>>,
}

pub struct Answer {
    run_status: JudgeStatus,
    time_elpased: u64,
    mem_used: u64,
}

impl Answer {
    fn new(run_status: JudgeStatus, time_elpased: u64, mem_used: u64) -> Answer {
        Answer {
            run_status,
            time_elpased,
            mem_used,
        }
    }

    pub fn get_run_status(&self) -> &JudgeStatus {
        &self.run_status
    }

    pub fn get_run_status_owned(self) -> JudgeStatus {
        self.run_status
    }

    pub fn get_time_elpased(&self) -> u64 {
        self.time_elpased
    }

    pub fn get_mem_used(&self) -> u64 {
        self.mem_used
    }
}

impl Default for Runner {
    fn default() -> Runner {
        let mut recipe: BTreeMap<String, Option<Vec<String>>> = BTreeMap::new();

        let config_text = match fs::read_to_string(CONFIG_PATH) {
            Ok(s) => s,
            Err(e) => panic!("config: `{CONFIG_PATH}` is missing: {e}"),
        };

        let data: Table = match toml::from_str(&config_text) {
            Ok(d) => d,
            Err(e) => panic!("config: Can't read from `{CONFIG_PATH}`: {e}"),
        };

        let lang_lst = match data.get("languages") {
            Some(v) => v,
            _ => panic!("config: [languages] should be set correctly"),
        };
        let execution_commands = match data.get("execute") {
            Some(v) => v,
            _ => panic!("config: [execute] should be set correctly"),
        };

        match lang_lst {
            Value::Array(lang_lst) => {
                for lang in lang_lst {
                    if let Value::String(lang) = lang {
                        recipe.insert(lang.clone(), None);
                    } else {
                        panic!("config: [languages] should be set correctly");
                    }
                }
            }
            _ => panic!("config: [languages] should be set correctly"),
        };

        match execution_commands {
            Value::Table(execution_commands) => {
                for (lang, val) in execution_commands.iter() {
                    if !recipe.contains_key(lang) {
                        continue;
                    }
                    let val = match val {
                        Value::String(val) => val,
                        _ => panic!("config: [execute] should be set correctly"),
                    };
                    let command_chain: Vec<String> = val
                        .split_ascii_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                    *recipe.get_mut(lang).unwrap() = Some(command_chain);
                }
            }
            _ => panic!("config: [execute] should be set correctly"),
        }

        dbg!(&recipe);

        Runner {
            running_recipe: Mutex::new(recipe),
        }
    }
}

impl Runner {
    fn generate_execution_command(
        &self,
        executable_path: &str,
        lang: &str,
    ) -> Result<Vec<CString>, Error> {
        let running_recipe = self.running_recipe.lock().unwrap();
        let command_chain = match running_recipe.get(lang) {
            Some(Some(chain)) => chain,
            _ => return Err(Error::LanguageNotFoundError),
        };

        let mut command = Vec::<CString>::new();
        for token in command_chain {
            let mut token: &str = token;
            if token.starts_with('$') {
                let (_, var) = token.split_at(1);
                match var {
                    "target" | "source" => token = executable_path, // FIXME tricky opt for interpreted languages, should write a better parser instead
                    _ => { /* do nothing */ }
                }
            }
            command.push(c_string!(token));
        }

        if command.is_empty() {
            return Err(Error::CommandEmptyError);
        }

        Ok(command)
    }

    pub fn execute(
        &self,
        executable_path: &str,
        lang: &str,
        testcases: &Vec<Testcase>,
    ) -> Result<Answer, Error> {
        let command = self.generate_execution_command(executable_path, lang)?;
        dbg!(&command);

        // TODO need to refactor
        // =====================================================================
        let r_mode = c_string!("r");
        let w_mode = c_string!("w");

        let mut wrong_cnt = 0usize;
        for (i, testcase) in testcases.iter().enumerate() {
            let input_testcase = &testcase.input_file;
            let stdout_testcase_file = TestcaseFile::new(
                format!("{}.stdout", input_testcase.get_name()).as_str(),
                format!("{}.stdout", input_testcase.get_path()).as_str(),
            );

            println!("===== Testing `{:?}`, id: {}", input_testcase, i);
            let errout_path = format!("{}.stderr", input_testcase.get_path());
            match unsafe { unistd::fork() } {
                Ok(ForkResult::Parent { child }) => {
                    waitpid(child, None).unwrap();
                }
                Ok(ForkResult::Child) => {
                    let input_path = c_string!(input_testcase.get_path());
                    let output_path =
                        c_string!(format!("{}.stdout", input_testcase.get_path()).as_str());
                    let errout_path = c_string!(errout_path.as_str());

                    unsafe {
                        let stdin = libc::fdopen(libc::STDIN_FILENO, r_mode.as_ptr());
                        libc::freopen(input_path.as_ptr(), r_mode.as_ptr(), stdin);
                        let stdout = libc::fdopen(libc::STDOUT_FILENO, w_mode.as_ptr());
                        libc::freopen(output_path.as_ptr(), w_mode.as_ptr(), stdout);
                        let stderr = libc::fdopen(libc::STDERR_FILENO, w_mode.as_ptr());
                        libc::freopen(errout_path.as_ptr(), w_mode.as_ptr(), stderr);
                    }

                    match unistd::execvp(&command[0], &command) {
                        Ok(_) => unreachable!(),
                        Err(errno) => unistd::write(
                            libc::STDERR_FILENO,
                            format!(
                                "Execvp error, errno = {:?}, input testcase file: {:?}\n",
                                errno, input_testcase
                            )
                            .as_bytes(),
                        )
                        .ok(),
                    };
                    unsafe {
                        libc::exit(0);
                    }
                }
                _ => panic!("Fork failed"),
            }

            // TODO clean stdout and stderr
            let err_content =
                fs::read_to_string(errout_path).map_err(|_| Error::FileSystemError)?;
            if !err_content.is_empty() {
                return Ok(Answer::new(JudgeStatus::RuntimeError(err_content), 0, 0));
            }

            // stdout => xxx.in.stdout
            // judge:
            match Judge::judge(&stdout_testcase_file, &testcase.output_file) {
                Ok(true) => {}
                Ok(false) => wrong_cnt += 1,
                Err(err) => println!("====? Errno: {}", err),
            };

            fs::remove_file(stdout_testcase_file.get_path()).map_err(|_| Error::FileSystemError)?;

            println!("===== Testcase {} was done", i);
        }
        // =====================================================================

        let run_state = if wrong_cnt == 0 {
            JudgeStatus::Accepted
        } else {
            JudgeStatus::WrongAnswer(testcases.len() - wrong_cnt, testcases.len())
        };
        // TODO
        let time_elpased = 1;
        let mem_used = 1;
        Ok(Answer::new(run_state, time_elpased, mem_used))
    }
}
