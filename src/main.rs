use std::{fs, collections::BTreeMap};

use coffee_oj_judger::{
    compiler::Compiler,
    file::{get_pairwise_testcase_files, TestcaseFile}, runner::Runner,
};

fn main() {
    println!("Hello, CoffeeOJ!");

    let read_path = "assets/p1001";

    let lst_read_dir = fs::read_dir(read_path);
    let mut testcase_files: Vec<TestcaseFile> = vec![];
    if let Ok(lst_read_dir) = lst_read_dir {
        for dir in lst_read_dir {
            let path = format!("{}", dir.unwrap().path().display());
            let sp = path.split_at(read_path.len() + 1);
            testcase_files.push(TestcaseFile::new(sp.1, &path));
        }
    }

    let pairwise_testcase_files = get_pairwise_testcase_files(testcase_files);
    for pair in pairwise_testcase_files {
        println!("{:?}", pair);
    }

    let new_runner: Runner = Default::default();
    let src_files: BTreeMap<&'static str, &'static str> = BTreeMap::from([
        ("test.cpp", "cpp"),
        ("test.rs", "rust"),
        ("test.py", "python"),
    ]);

    for (src_file, lang) in src_files.iter() {
        println!();
        let src_path = format!("assets/src/{}", src_file);
        if new_runner.execute(&src_path, lang).is_ok() {
            println!("{} exec succeed!!", src_file);
        }
    }
}
