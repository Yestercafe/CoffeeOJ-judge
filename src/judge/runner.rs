use std::{
    collections::BTreeMap,
    ffi::CString,
    fmt, fs,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{self, fork, ForkResult},
};
use toml::{Table, Value};

use crate::{
    c_string,
    judge::{consts::CONFIG_PATH, file::Testcase},
};

use super::comparer::{self, Comparer, ComparerResult};

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
    IOError,
}

pub struct Runner {
    running_recipe: Mutex<BTreeMap<String, Option<Vec<String>>>>,
}

type DeferFn = Box<dyn FnOnce(Arc<RunnerJobSharedData>) + Send + 'static>;

struct RunnerJobSharedData {
    cnt_testcases: AtomicUsize,
    cnt_checked: AtomicUsize,
    cnt_wrong_answer: AtomicUsize,
    mem_cost: AtomicU64,
    time_cost: AtomicU64,
    // FIXME shared_data may be deadlocked?
    defer: Mutex<Option<DeferFn>>,
    executable_path: Mutex<String>,
    command: Mutex<Vec<CString>>,
}

impl RunnerJobSharedData {
    pub fn get_cnt_testcases(&self) -> usize {
        self.cnt_checked.load(Ordering::SeqCst)
    }

    pub fn get_cnt_checked(&self) -> usize {
        self.cnt_checked.load(Ordering::SeqCst)
    }

    pub fn get_cnt_wrong_answer(&self) -> usize {
        self.cnt_wrong_answer.load(Ordering::SeqCst)
    }

    pub fn get_mem_cost(&self) -> u64 {
        self.mem_cost.load(Ordering::SeqCst)
    }

    pub fn get_time_cost(&self) -> u64 {
        self.time_cost.load(Ordering::SeqCst)
    }
}

impl fmt::Debug for RunnerJobSharedData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunnerJobSharedData")
            .field("cnt_testcases", &self.get_cnt_testcases())
            .field("cnt_checked", &self.get_cnt_checked())
            .field("cnt_wrong_answer", &self.get_cnt_wrong_answer())
            .field("mem_cost", &self.get_mem_cost())
            .field("time_cost", &self.get_time_cost())
            .finish()
    }
}

impl RunnerJobSharedData {
    fn new(
        cnt_testcases: usize,
        defer: DeferFn,
        execuable_path: String,
        command: Vec<CString>,
    ) -> RunnerJobSharedData {
        RunnerJobSharedData {
            cnt_testcases: AtomicUsize::new(cnt_testcases),
            cnt_checked: AtomicUsize::new(0usize),
            cnt_wrong_answer: AtomicUsize::new(0usize),
            mem_cost: AtomicU64::new(0u64),
            time_cost: AtomicU64::new(0u64),
            defer: Mutex::new(Some(defer)),
            executable_path: Mutex::new(execuable_path),
            command: Mutex::new(command),
        }
    }

    fn is_complete(&self) -> Option<DeferFn> {
        let cnt_checked = self.cnt_checked.load(Ordering::SeqCst);
        if cnt_checked < self.cnt_testcases.load(Ordering::Relaxed) {
            return None;
        }

        // FIXME stupid implementation, and I am not sure if it works
        let mut defer = self.defer.lock().unwrap();
        let ret = if defer.deref().is_some() {
            std::mem::take(defer.deref_mut()).unwrap()
        } else {
            return None;
        };

        Some(ret)
    }
}

#[derive(Debug)]
pub struct RunnerJob {
    testcase: Testcase,
    shared_data: Arc<RunnerJobSharedData>,
}

impl RunnerJob {
    fn new(testcase: Testcase, shared_data: Arc<RunnerJobSharedData>) -> RunnerJob {
        RunnerJob {
            testcase,
            shared_data,
        }
    }

    pub fn execute_once(self) -> Result<(), Error> {
        let exec_stdout_path = format!(
            "{}-{}-stdout",
            *self.shared_data.executable_path.lock().unwrap(),
            self.testcase.output_file.get_name()
        );
        let exec_stderr_path = format!(
            "{}-{}-stderr",
            *self.shared_data.executable_path.lock().unwrap(),
            self.testcase.output_file.get_name()
        );
        let Testcase {
            input_file,
            output_file,
        } = &self.testcase;
        let testcase_input_path = input_file.get_path();
        let testcase_output_path = output_file.get_path();

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                let testcase_input_path = c_string!(testcase_input_path);
                let exec_stdout_path = c_string!(exec_stdout_path.as_str());
                let exec_stderr_path = c_string!(exec_stderr_path.as_str());
                let r_mode = c_string!("r");
                let w_mode = c_string!("w");

                unsafe {
                    let stdin = libc::fdopen(libc::STDIN_FILENO, r_mode.as_ptr());
                    libc::freopen(testcase_input_path.as_ptr(), r_mode.as_ptr(), stdin);
                    let stdout = libc::fdopen(libc::STDOUT_FILENO, w_mode.as_ptr());
                    libc::freopen(exec_stdout_path.as_ptr(), w_mode.as_ptr(), stdout);
                    let stderr = libc::fdopen(libc::STDERR_FILENO, w_mode.as_ptr());
                    libc::freopen(exec_stderr_path.as_ptr(), w_mode.as_ptr(), stderr);
                }

                let command = self.shared_data.command.lock().unwrap();
                let command = command.deref();
                match unistd::execvp(&command[0], command) {
                    Ok(_) => unreachable!(),
                    Err(errno) => unistd::write(
                        libc::STDERR_FILENO,
                        format!(
                            "Execvp error, errno = {:?}, testcase input file path: {:?}\n",
                            errno, testcase_input_path
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

        let result = Comparer::new(testcase_output_path, &exec_stdout_path)
            .compare()
            .map_err(|e| match e {
                comparer::Error::FileSystemError => Error::FileSystemError,
                comparer::Error::IOError => Error::IOError,
            })?;
        // TODO do clean

        if result != ComparerResult::Consistent {
            self.shared_data
                .cnt_wrong_answer
                .fetch_add(1, Ordering::SeqCst);
        }

        // TODO mem and time
        let mem_used = 1u64;
        let time_elapsed = 1u64;

        self.shared_data
            .mem_cost
            .fetch_add(mem_used, Ordering::SeqCst);
        self.shared_data
            .time_cost
            .fetch_add(time_elapsed, Ordering::SeqCst);

        self.shared_data.cnt_checked.fetch_add(1, Ordering::SeqCst);

        dbg!(&result);

        if let Some(defer) = self.shared_data.is_complete() {
            defer(self.shared_data.clone());
        }

        Ok(())
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
        executable_path: String,
        lang: &str,
        testcases: &Vec<Testcase>,
    ) -> Result<Vec<RunnerJob>, Error> {
        let command = self.generate_execution_command(&executable_path, lang)?;
        dbg!(&*command);

        let defer = Box::new(|shared_data: Arc<RunnerJobSharedData>| {
            println!("{:#?}", *shared_data);
            // TODO write into Database
        });
        let shared_data = Arc::new(RunnerJobSharedData::new(
            testcases.len(),
            defer,
            executable_path,
            command,
        ));

        let mut runner_jobs = vec![];
        for (_, testcase) in testcases.iter().enumerate() {
            let runner_job = RunnerJob::new(testcase.clone(), shared_data.clone());
            runner_jobs.push(runner_job);
        }

        Ok(runner_jobs)
    }
}
