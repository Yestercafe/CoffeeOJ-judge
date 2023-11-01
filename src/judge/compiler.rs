#![allow(clippy::missing_safety_doc)]

use std::{
    collections::{BTreeMap, HashMap},
    ffi::CString,
    fmt::Debug,
    fs, path,
};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{execvp, fork, write, ForkResult},
};
use toml::{Table, Value};

use crate::{c_string, c_string_ptr, judge::runner::RunnerErr};

use super::file::SavedSource;

pub struct Compiler {
    pub compilers: BTreeMap<String, Vec<String>>,
}

impl Compiler {
    fn generate_compilation_instruction(
        &self,
        src_path: &str,
        lang: &str,
    ) -> Result<Vec<String>, RunnerErr> {
        let path = path::Path::new(&src_path);
        if !path.exists() {
            return Err(RunnerErr::MissingConfig);
        }

        let target_path = format!("{}{}", src_path, crate::judge::runner::EXECUABLE_SUFFIX);
        let instrs = match self.compilers.get(lang) {
            Some(ins) => ins.clone(),
            None => {
                return Err(RunnerErr::MissingCompConfig(format!(
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

    fn get_file_content(path: &str) -> Option<String> {
        match fs::read_to_string(path) {
            Ok(content) => {
                if content.is_empty() {
                    None
                } else {
                    Some(content)
                }
            }
            Err(err) => panic!("{:?}: maybe missing file {:?}?", err, path),
        }
    }

    pub fn compile(&self, src_path: &str, lang: &str) -> Result<(), RunnerErr> {
        const COMPILE_OUTPUT: &str = "output.txt";

        let instrs = match self.generate_compilation_instruction(src_path, lang) {
            Ok(instrs) => instrs,
            Err(err) => return Err(err),
        };

        let instrs = instrs
            .iter()
            .map(|rstr| CString::new(rstr.as_str()).unwrap())
            .collect::<Vec<_>>();

        println!("{:?}", instrs);

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                let output_filename = CString::new(COMPILE_OUTPUT).unwrap();
                let w_mode = CString::new("w").unwrap();

                // redirect stderr to file
                unsafe {
                    let stdout = libc::fdopen(libc::STDERR_FILENO, w_mode.as_ptr());
                    libc::freopen(output_filename.as_ptr(), w_mode.as_ptr(), stdout);
                }

                match execvp(&instrs[0], &instrs) {
                    Ok(_) => unreachable!(),
                    Err(errno) => write(
                        libc::STDERR_FILENO,
                        format!("Execvp error, errno = {:?}\n", errno).as_bytes(),
                    )
                    .ok(),
                };
                unsafe {
                    libc::exit(0);
                }
            }
            Err(_) => println!("Fork failed"),
        }

        println!("write out > output.txt");

        let ret = if let Some(info) = Self::get_file_content(COMPILE_OUTPUT) {
            Err(RunnerErr::CompErr(format!("Compilation error: {}", info)))
        } else {
            Ok(())
        };

        fs::remove_file(COMPILE_OUTPUT)
            .map_err(|_| RunnerErr::UnknownErr("can't delete the compile output file".into()))?;

        ret
    }
}

impl Default for Compiler {
    fn default() -> Self {
        let config_str = match fs::read_to_string(crate::judge::runner::CONFIG_PATH) {
            Ok(string) => string,
            Err(err) => panic!("Config missing: {}", err),
        };
        let config: Table = toml::from_str(&config_str).unwrap();
        let compiler_table = &config["compilers"];
        let mut compilers: BTreeMap<String, Vec<String>> = BTreeMap::new();
        match compiler_table {
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
                            compilers.insert(item.0.clone(), arg_lst);
                        }
                        _ => panic!("Error compiler arguments structure in `config.toml`, should be an array")
                    }
                }
            }
            _ => panic!(
                "Error token `compilers` structure in file `config.toml`, should be a Table."
            ),
        }

        Self { compilers }
    }
}

impl Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for kv in self.compilers.iter() {
            f.write_str(format!("{:?} {:?}\n", kv.0, kv.1).as_str())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    LanguageNotFoundError,
    ForkFailed,
    NoCompilationLogError,
    CompilationError(String),
}

pub struct Compiler2 {
    pub compiling_recipe: HashMap<String, Vec<String>>,
}

impl Compiler2 {
    fn new() -> Compiler2 {
        Compiler2 {
            compiling_recipe: HashMap::new(),
        }
    }

    fn generate_compilation_command(
        &self,
        source: &SavedSource,
        lang: &str,
    ) -> Result<(String, Vec<CString>), Error> {
        let command_chain = match self.compiling_recipe.get(lang) {
            Some(chain) => chain,
            _ => return Err(Error::LanguageNotFoundError),
        };
        let target_full_path = format!("{}.exe", source.get_full_path());

        let mut command = Vec::<CString>::new();
        for token in command_chain {
            let mut token: &str = token;
            if token.starts_with("$") {
                let (_, var) = token.split_at(0);
                match var {
                    "source" => token = source.get_full_path(),
                    "target" => token = &target_full_path,
                    _ => { /* do nothing */ }
                }
            }
            command.push(c_string!(token));
        }

        Ok((target_full_path, command))
    }

    pub fn compile(&self, source: &SavedSource, lang: &str) -> Result<String, Error> {
        let (target_full_path, command) = self.generate_compilation_command(source, lang)?;
        if command.is_empty() {
            return Ok(String::from(source.get_full_path()));
        }
        dbg!(&command);
        let log_path = format!("{target_full_path}.log");

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                let log_path = c_string_ptr!(log_path);
                let w_mode = c_string_ptr!("w");

                unsafe {
                    // open a log file to read in compilation log
                    let log_output = libc::fdopen(libc::STDERR_FILENO, w_mode);
                    libc::freopen(log_path, w_mode, log_output);
                }

                match execvp(&command[0], &command) {
                    Ok(_) => unreachable!(),
                    Err(errno) => write(
                        libc::STDERR_FILENO,
                        format!("Execvp error, errno = {:?}\n", errno).as_bytes(),
                    )
                    .ok(),
                };

                unsafe { libc::exit(0) };
            }
            _ => return Err(Error::ForkFailed),
        }

        let log_content = fs::read_to_string(log_path).map_err(|_| Error::NoCompilationLogError)?;
        if log_content.is_empty() {
            Ok(target_full_path)
        } else {
            Err(Error::CompilationError(log_content))
        }
    }
}
