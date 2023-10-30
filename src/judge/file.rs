use std::{collections::BTreeMap, fmt::Debug};

#[derive(Clone)]
pub struct TestcaseFile {
    filename: String,
    path: String,
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
