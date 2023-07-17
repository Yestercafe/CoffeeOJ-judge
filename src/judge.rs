use std::{fmt::Debug, fs};

use crate::{file::TestcaseFile, runner::RunnerErr};

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub enum JudgeErr {
    // WrongAnswer(#passed testcases, #testcases)
    WrongAnswer(usize, usize),
    RuntimeError(String),
    CompilationError(String),
    InternalError(RunnerErr),
}

pub struct Differ {
    lhs_path: String,
    rhs_path: String,
    diffs: Vec<(usize, String, usize, String)>,
}

impl Differ {
    pub fn new(lhs_path: &str, lhs: Vec<&str>, rhs_path: &str, rhs: Vec<&str>) -> Self {
        println!("===lhs: {:?}", lhs);
        println!("===rhs: {:?}", rhs);

        let (mut i, mut j) = (0usize, 0usize);
        let mut diffs: Vec<(usize, String, usize, String)> = vec![];
        while i < lhs.len() && j < rhs.len() {
            while i < lhs.len() && lhs[i].is_empty() {
                println!("---i: {:?}", i);
                i += 1;
            }
            if i == lhs.len() {
                break;
            }
            while j < rhs.len() && rhs[j].is_empty() {
                j += 1;
            }
            if j == rhs.len() {
                break;
            }

            println!("---comp: {:?} v.s. {:?}", lhs[i], rhs[i]);
            if lhs[i] != rhs[j] {
                diffs.push((i, String::from(lhs[i]), j, String::from(rhs[j])));
                return Self {
                    lhs_path: String::from(lhs_path),
                    rhs_path: String::from(rhs_path),
                    diffs,
                };
            }
            i += 1;
            j += 1;
        }
        while i < lhs.len() && lhs[i].is_empty() {
            i += 1;
        }
        while j < rhs.len() && rhs[j].is_empty() {
            j += 1;
        }

        if i != lhs.len() {
            diffs.push((i, String::from(lhs[i]), j, String::from("(eof)")));
        }
        if j != rhs.len() {
            diffs.push((i, String::from("(eof)"), j, String::from(rhs[j])));
        }

        Self {
            lhs_path: String::from(lhs_path),
            rhs_path: String::from(rhs_path),
            diffs,
        }
    }
}

impl Debug for Differ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for diff in &self.diffs {
            f.write_str(format!(">>> In {}:{}, {}\n", self.lhs_path, diff.0, diff.1).as_str())?;
            f.write_str(format!("<<< In {}:{}, {}", self.rhs_path, diff.2, diff.3).as_str())?;
        }
        Ok(())
    }
}

pub struct Judge {}

impl Judge {
    pub fn judge(
        stdout_file: &TestcaseFile,
        preset_output_file: &TestcaseFile,
    ) -> Result<bool, usize> {
        let stdout_ctt = match fs::read_to_string(stdout_file.get_path()) {
            Ok(ctt) => ctt,
            Err(_) => return Err(0),
        };
        let stdout_ctt = stdout_ctt.split('\n').collect::<Vec<_>>();
        let preset_ctt = match fs::read_to_string(preset_output_file.get_path()) {
            Ok(ctt) => ctt,
            Err(_) => return Err(1),
        };
        let preset_ctt = preset_ctt.split('\n').collect::<Vec<_>>();

        let differ = Differ::new(
            stdout_file.get_path(),
            stdout_ctt,
            preset_output_file.get_path(),
            preset_ctt,
        );
        if differ.diffs.is_empty() {
            Ok(true)
        } else {
            println!("{:?}", differ);
            Ok(false)
        }
    }
}
