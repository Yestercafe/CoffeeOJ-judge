use std::{collections::BTreeMap, ffi::CString, fs, path};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{self, ForkResult},
};
use toml::{Table, Value};

use crate::compiler::Compiler;

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

        let target_path = format!("./{}{}", src_path, crate::runner::EXECUABLE_SUFFIX);
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

    pub fn execute(&self, src_path: &str, lang: &str) -> Result<(), RunnerErr> {
        let compiler_ret = self.compiler.compile(src_path, lang);
        if let Err(RunnerErr::MissingConfig) = compiler_ret {
            return compiler_ret;
        } else if let Err(RunnerErr::MissingCompConfig(_)) = compiler_ret {
            println!("Lang `{}` doesn't need to compile, run directly.", lang);
        } else if let Err(RunnerErr::CompErr(info)) = compiler_ret {
            return Err(RunnerErr::CompErr(info));
        }

        let gen_exec_ret = self.generate_execution_instruction(src_path, lang);
        let exec_instrs = match gen_exec_ret {
            Ok(instrs) => instrs,
            Err(e) => return Err(e),
        }
        .iter()
        .map(|rstr| CString::new(rstr.as_str()).unwrap())
        .collect::<Vec<_>>();

        println!("{:?}", exec_instrs);
        match unsafe { unistd::fork() } {
            Ok(ForkResult::Parent { child }) => {
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                match unistd::execvp(&exec_instrs[0], &exec_instrs) {
                    Ok(_) => unreachable!(),
                    Err(errno) => unistd::write(
                        libc::STDERR_FILENO,
                        format!("Execvp error, errno = {:?}\n", errno).as_bytes(),
                    )
                    .ok(),
                };
                unsafe {
                    libc::exit(0);
                }
            }
            _ => panic!("Fork failed"),
        }

        Ok(())
    }
}

impl Default for Runner {
    fn default() -> Self {
        let config_str = match fs::read_to_string(crate::runner::CONFIG_PATH) {
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
