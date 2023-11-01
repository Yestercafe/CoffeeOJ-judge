use std::{collections::BTreeMap, ffi::CString, fs, path};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{self, fork, ForkResult},
};
use toml::{Table, Value};

use crate::{
    c_string, c_string_ptr,
    judge::{
        compiler::Compiler,
        file::{Testcase, TestcaseFile},
        judge::{Judge, JudgeErr},
    },
};

pub static CONFIG_PATH: &str = "config.toml";
pub static EXECUABLE_SUFFIX: &str = ".exe";

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum RunnerErr {
    MissingConfig,
    MissingCompConfig(String),
    MissingExecConfig(String),
    CompErr(String),
    UnknownErr(String),
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum FileType {
    Stdout,
    Stdin,
    Stderr,
    File(String),
}

pub struct Runner {
    compiler: Compiler,
    run_recipe: BTreeMap<String, Vec<String>>,
}

impl Runner {
    fn generate_execution_instruction(
        &self,
        src_path: &str,
        lang: &str,
    ) -> Result<Vec<String>, RunnerErr> {
        let path = path::Path::new(&src_path);
        if !path.exists() {
            return Err(RunnerErr::MissingConfig);
        }

        let target_path = format!("./{}{}", src_path, crate::judge::runner::EXECUABLE_SUFFIX);
        let instrs = match self.run_recipe.get(lang) {
            Some(ins) => ins.clone(),
            None => {
                return Err(RunnerErr::MissingExecConfig(format!(
                    "Lang `{}` is not supported.",
                    lang
                )))
            }
        };

        Ok(instrs
            .iter()
            .map(|instr| {
                if instr.starts_with('$') {
                    match instr.split_at(1).1 {
                        "target" => target_path.clone(),
                        "source" => src_path.into(),
                        _ => panic!("never reach"),
                    }
                } else {
                    instr.clone()
                }
            })
            .collect::<Vec<_>>())
    }

    pub fn execute(
        &self,
        src_path: &str,
        lang: &str,
        testcases: &Vec<Testcase>,
    ) -> Result<(), JudgeErr> {
        let compiler_ret = self.compiler.compile(src_path, lang);
        let mut is_interpret_lang = false;
        if let Err(RunnerErr::MissingConfig) = compiler_ret {
            return Err(JudgeErr::InternalError(RunnerErr::MissingConfig));
        } else if let Err(RunnerErr::MissingCompConfig(_)) = compiler_ret {
            println!("Lang `{}` doesn't need to compile, run directly.", lang);
            is_interpret_lang = true;
        } else if let Err(RunnerErr::CompErr(info)) = compiler_ret {
            return Err(JudgeErr::CompilationError(info));
        }

        let gen_exec_ret = self.generate_execution_instruction(src_path, lang);
        let exec_instrs = match gen_exec_ret {
            Ok(instrs) => instrs,
            Err(e) => return Err(JudgeErr::InternalError(e)),
        }
        .iter()
        .map(|rstr| CString::new(rstr.as_str()).unwrap())
        .collect::<Vec<_>>();

        println!("{:?}", exec_instrs);

        let mut wrong_cnt = 0usize;
        for (i, testcase) in testcases.iter().enumerate() {
            let input_testcase = &testcase.input_file;
            let stdout_testcase_file = TestcaseFile::new(
                format!("{}.stdout", input_testcase.get_name()).as_str(),
                format!("{}.stdout", input_testcase.get_path()).as_str(),
            );

            println!("===== Testing `{:?}`, id: {}", input_testcase, i);
            match unsafe { unistd::fork() } {
                Ok(ForkResult::Parent { child }) => {
                    waitpid(child, None).unwrap();
                }
                Ok(ForkResult::Child) => {
                    let input_path = CString::new(input_testcase.get_path()).unwrap();
                    let output_path =
                        CString::new(format!("{}.stdout", input_testcase.get_path())).unwrap();
                    let r_mode = CString::new("r").unwrap();
                    let w_mode = CString::new("w").unwrap();

                    unsafe {
                        let stdin = libc::fdopen(libc::STDIN_FILENO, r_mode.as_ptr());
                        libc::freopen(input_path.as_ptr(), r_mode.as_ptr(), stdin);
                        let stdout = libc::fdopen(libc::STDOUT_FILENO, w_mode.as_ptr());
                        libc::freopen(output_path.as_ptr(), w_mode.as_ptr(), stdout);
                    }

                    match unistd::execvp(&exec_instrs[0], &exec_instrs) {
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

            // stdout => xxx.in.stdout
            // judge:
            match Judge::judge(&stdout_testcase_file, &testcase.output_file) {
                Ok(true) => {}
                Ok(false) => wrong_cnt += 1,
                Err(err) => println!("====? Errno: {}", err),
            };

            fs::remove_file(stdout_testcase_file.get_path()).map_err(|_| {
                JudgeErr::InternalError(RunnerErr::UnknownErr(format!(
                    "can't delete file `{}`",
                    stdout_testcase_file.get_path()
                )))
            })?;

            println!("===== Testcase {} was done", i);
        }

        if !is_interpret_lang {
            let target_path = format!("./{}{}", src_path, crate::judge::runner::EXECUABLE_SUFFIX);
            let _ = fs::remove_file(target_path);
        }

        if wrong_cnt == 0 {
            Ok(())
        } else {
            Err(JudgeErr::WrongAnswer(
                testcases.len() - wrong_cnt,
                testcases.len(),
            ))
        }
    }
}

impl Default for Runner {
    fn default() -> Self {
        let config_str = match fs::read_to_string(crate::judge::runner::CONFIG_PATH) {
            Ok(string) => string,
            Err(err) => panic!("{}", err),
        };
        let config: Table = toml::from_str(&config_str).unwrap();
        let execute_table = &config["execute"];

        let mut run_recipe: BTreeMap<String, Vec<String>> = BTreeMap::new();

        match execute_table {
            Value::Table(table) => {
                for item in table.iter() {
                    match item.1 {
                        Value::Array(args) => {
                            let mut arg_lst: Vec<String> = vec![];
                            for arg in args {
                                if let Value::String(arg) = arg {
                                    arg_lst.push(arg.clone());
                                }
                            }
                            run_recipe.insert(item.0.clone(), arg_lst);
                        }
                        _ => panic!("Error execute arguments structure in `config.toml`, should be an array")
                    }
                }
            }
            _ => {
                panic!("Error token `execute` structure in file `config.toml`, should be a Table.")
            }
        }

        Self {
            compiler: Default::default(),
            run_recipe,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    ForkFailed,
    LanguageNotFoundError,
    CommandEmptyError,
    FileSystemError,
}

pub struct Runner2 {
    running_recipe: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Copy)]
pub enum RunStatus {
    AC,
    WA(usize, usize),
    TLE,
    MLE,
    RE,
    Unknown,
}

pub struct Answer {
    run_status: RunStatus,
    time_elpased: u64,
    mem_used: u64,
}

impl Answer {
    fn new(run_status: RunStatus, time_elpased: u64, mem_used: u64) -> Answer {
        Answer {
            run_status,
            time_elpased,
            mem_used,
        }
    }

    pub fn get_run_status(&self) -> RunStatus {
        self.run_status
    }

    pub fn get_time_elpased(&self) -> u64 {
        self.time_elpased
    }

    pub fn get_mem_used(&self) -> u64 {
        self.mem_used
    }
}

impl Runner2 {
    fn new() -> Runner2 {
        Runner2 {
            running_recipe: BTreeMap::new(),
        }
    }

    fn generate_execution_command(
        &self,
        executable_path: &str,
        lang: &str,
    ) -> Result<Vec<CString>, Error> {
        let command_chain = match self.running_recipe.get(lang) {
            Some(chain) => chain,
            _ => return Err(Error::LanguageNotFoundError),
        };

        let mut command = Vec::<CString>::new();
        for token in command_chain {
            let mut token: &str = token;
            if token.starts_with("$") {
                let (_, var) = token.split_at(0);
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
        let mut wrong_cnt = 0usize;
        for (i, testcase) in testcases.iter().enumerate() {
            let input_testcase = &testcase.input_file;
            let stdout_testcase_file = TestcaseFile::new(
                format!("{}.stdout", input_testcase.get_name()).as_str(),
                format!("{}.stdout", input_testcase.get_path()).as_str(),
            );

            println!("===== Testing `{:?}`, id: {}", input_testcase, i);
            match unsafe { unistd::fork() } {
                Ok(ForkResult::Parent { child }) => {
                    waitpid(child, None).unwrap();
                }
                Ok(ForkResult::Child) => {
                    let input_path = c_string_ptr!(input_testcase.get_path());
                    let output_path =
                        c_string_ptr!(format!("{}.stdout", input_testcase.get_path()));
                    let r_mode = c_string_ptr!("r");
                    let w_mode = c_string_ptr!("w");

                    unsafe {
                        let stdin = libc::fdopen(libc::STDIN_FILENO, r_mode);
                        libc::freopen(input_path, r_mode, stdin);
                        let stdout = libc::fdopen(libc::STDOUT_FILENO, w_mode);
                        libc::freopen(output_path, w_mode, stdout);
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
            RunStatus::AC
        } else {
            RunStatus::WA(testcases.len() - wrong_cnt, testcases.len())
        };
        // TODO
        let time_elpased = 1;
        let mem_used = 1;
        Ok(Answer::new(run_state, time_elpased, mem_used))
    }
}
