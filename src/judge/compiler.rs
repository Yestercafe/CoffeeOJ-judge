#![allow(clippy::missing_safety_doc)]

use std::{collections::HashMap, ffi::CString, fmt::Debug, fs, sync::Mutex};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{execvp, fork, write, ForkResult},
};
use toml::{Table, Value};

use crate::c_string;

use super::{consts::CONFIG_PATH, file::SavedSource};

#[derive(Debug)]
pub enum Error {
    LanguageNotFoundError,
    ForkFailed,
    NoCompilationLogError,
    CompilationError(String),
}

pub struct Compiler {
    pub compiling_recipe: Mutex<HashMap<String, Option<Vec<String>>>>,
}

impl Default for Compiler {
    fn default() -> Compiler {
        let mut recipe: HashMap<String, Option<Vec<String>>> = HashMap::new();
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
        let compilation_commands = match data.get("compile") {
            Some(v) => v,
            _ => panic!("config: [compile] should be set correctly"),
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

        match compilation_commands {
            Value::Table(compilation_commands) => {
                for (lang, val) in compilation_commands.iter() {
                    if !recipe.contains_key(lang) {
                        continue;
                    }
                    let val = match val {
                        Value::String(val) => val,
                        _ => panic!("config: [compile] should be set correctly"),
                    };
                    let command_chain: Vec<String> = val
                        .split_ascii_whitespace()
                        .map(|s| s.to_string())
                        .collect();
                    *recipe.get_mut(lang).unwrap() = Some(command_chain);
                }
            }
            _ => panic!("config: [compile] should be set correctly"),
        }

        Compiler {
            compiling_recipe: Mutex::new(recipe),
        }
    }
}

impl Compiler {
    fn generate_compilation_command(
        &self,
        source: &SavedSource,
        lang: &str,
    ) -> Result<Option<(String, Vec<CString>)>, Error> {
        let compiling_recipe = self.compiling_recipe.lock().unwrap();
        let command_chain = match compiling_recipe.get(lang) {
            Some(chain) => chain,
            _ => return Err(Error::LanguageNotFoundError),
        };
        let command_chain = match command_chain {
            Some(c) => c,
            _ => return Ok(None),
        };

        let target_full_path = format!("{}.exe", source.get_full_path());

        let mut command = Vec::<CString>::new();
        for token in command_chain {
            let mut token: &str = token;
            if token.starts_with('$') {
                let (_, var) = token.split_at(1);
                match var {
                    "source" => token = source.get_full_path(),
                    "target" => token = &target_full_path,
                    _ => { /* do nothing */ }
                }
            }
            command.push(c_string!(token));
        }

        Ok(Some((target_full_path, command)))
    }

    pub fn compile(&self, source: &SavedSource, lang: &str) -> Result<String, Error> {
        let ret = self.generate_compilation_command(source, lang)?;
        if ret.is_none() {
            return Ok(source.get_full_path().to_string());
        }
        let (target_full_path, command) = ret.unwrap();
        // dbg!(&command);
        let log_path = format!("{target_full_path}.log");
        // dbg!(&log_path);

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                // let log_path = c_string_ptr!(log_path);
                let log_path = c_string!(log_path.as_str());
                let w_mode = c_string!("w");

                unsafe {
                    // open a log file to read in compilation log
                    let log_output = libc::fdopen(libc::STDERR_FILENO, w_mode.as_ptr());
                    libc::freopen(log_path.as_ptr(), w_mode.as_ptr(), log_output);
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
