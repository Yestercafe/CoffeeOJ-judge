use std::{collections::BTreeMap, fmt::Debug, fs::File, io::Write};

use super::{
    consts::{LANG_EXTENSIONS, SOURCE_CODE_SAVED_PATH},
    JudgeStatus,
};

use random_number::{self, random};

// TODO binds an Arc of Task
#[derive(Default)]
pub struct DiffTask {}

impl DiffTask {
    pub fn do_diff(self) -> JudgeStatus {
        // TODO
        JudgeStatus::Halt
    }
}

#[derive(Clone)]
pub struct TestcaseFile {
    filename: String,
    path: String,
}

pub enum Error {
    FileNotFound(String),
    ParseError(String),
}

impl TestcaseFile {
    pub fn new(filename: &str, path: &str) -> Self {
        Self {
            filename: filename.into(),
            path: path.into(),
        }
    }

    pub fn get_name(&self) -> &str {
        return self.filename.as_str();
    }

    pub fn get_path(&self) -> &str {
        return self.path.as_str();
    }
}

impl Debug for TestcaseFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!("[filename: \"{}\", path: \"{}\"]", self.filename, self.path).as_str(),
        )?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct Testcase {
    pub input_file: TestcaseFile,
    pub output_file: TestcaseFile,
}

impl Testcase {
    fn new(input_file: TestcaseFile, output_file: TestcaseFile) -> Self {
        Self {
            input_file,
            output_file,
        }
    }
}

impl Debug for Testcase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            format!(
                "[input_file: {:?}, output_file: {:?}]",
                self.input_file, self.output_file
            )
            .as_str(),
        )?;
        Ok(())
    }
}

pub fn get_pairwise_testcase_files(testcase_files: Vec<TestcaseFile>) -> Vec<Testcase> {
    let mut input_file_table: BTreeMap<String, usize> = BTreeMap::new();
    let mut output_file_table: BTreeMap<String, usize> = BTreeMap::new();

    for (i, file) in testcase_files.iter().enumerate() {
        if file.filename.ends_with(".in") {
            let trim_len = ".in".len();
            let main_name = file.filename.split_at(file.filename.len() - trim_len).0;
            input_file_table.insert(main_name.into(), i);
        } else if file.filename.ends_with(".out") {
            let trim_len = ".out".len();
            let main_name = file.filename.split_at(file.filename.len() - trim_len).0;
            output_file_table.insert(main_name.into(), i);
        }
    }

    let mut ret: Vec<Testcase> = vec![];
    for (input_file_main_name, input_file_idx) in input_file_table.iter() {
        if let Some(output_kv) = output_file_table.get_key_value(input_file_main_name) {
            ret.push(Testcase::new(
                testcase_files[*input_file_idx].clone(),
                testcase_files[*output_kv.1].clone(),
            ));
        }
    }

    ret
}

pub struct SavedSource {
    submission_id: u64,
    full_path: String,
}

impl SavedSource {
    fn new(submission_id: u64, full_path: String) -> SavedSource {
        SavedSource {
            submission_id,
            full_path,
        }
    }

    pub fn get_submission_id(&self) -> u64 {
        self.submission_id
    }

    pub fn get_full_path(&self) -> &str {
        &self.full_path
    }
}

pub fn save_source_code(source_code: &str, lang: &str) -> Result<SavedSource, String> {
    let ext = match LANG_EXTENSIONS.get(lang) {
        Some(ext) => ext,
        None => return Err("Unsupported language".to_string()), // TODO handle as a real error
    };

    // TODO get auto-increased id from SQL
    let submission_id: u64 = random!(0..1000000u64);

    let filename = format!("{submission_id}.{ext}");
    let full_path = format!("{}/{filename}", SOURCE_CODE_SAVED_PATH);
    let mut file = File::create(&full_path).map_err(|_| format!("Can't create file {filename}"))?;
    // TODO maybe use buf writer?
    file.write_all(source_code.as_bytes())
        .map_err(|_| "Can't write to file".to_string())?;

    Ok(SavedSource::new(submission_id, full_path))
}
