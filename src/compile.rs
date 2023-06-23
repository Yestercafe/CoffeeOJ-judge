#![allow(clippy::missing_safety_doc)]

use std::{collections::BTreeMap, ffi::CString, fmt::Debug, fs, path};

use nix::{
    libc,
    sys::wait::waitpid,
    unistd::{execvp, fork, write, ForkResult},
};
use toml::{Table, Value};

static COMPILER_CONFIG_PATH: &str = "compiler_config.toml";

pub struct Compiler {
    pub compilers: BTreeMap<String, Vec<String>>,
}

impl Compiler {
    fn generate_compilation_instruction(&self, src_path: &str, lang: &str) -> Vec<String> {
        let path = path::Path::new(&src_path);
        if !path.exists() {
            panic!("`{}` doesn't exist.", src_path)
        }

        let target_path = format!("{}.exe", src_path);
        let instrs = match self.compilers.get(lang) {
            Some(ins) => ins.clone(),
            None => panic!("Lang `{}` is not supported.", lang),
        };

        instrs
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
            .collect::<Vec<_>>()
    }

    pub fn compile(&self, src_path: &str, lang: &str) {
        let instrs = self
            .generate_compilation_instruction(src_path, lang)
            .iter()
            .map(|rstr| CString::new(rstr.as_str()).unwrap())
            .collect::<Vec<_>>();
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child, .. }) => {
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                let output_filename = CString::new("output.txt").unwrap();
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
    }
}

impl Default for Compiler {
    fn default() -> Self {
        let config_str = match fs::read_to_string(COMPILER_CONFIG_PATH) {
            Ok(string) => string,
            Err(err) => panic!("{}", err),
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
                        _ => panic!("Error compiler arguments structure in `compiler_config.toml`, should be an array")
                    }
                }
            }
            _ => panic!("Error token `compilers` structure in file `compiler_config.toml`, should be a Table.")
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
