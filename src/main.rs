use std::fs::{self};

use coffee_oj_judger::{
    compile::Compiler,
    file::{get_pairwise_testcase_files, TestcaseFile},
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

    let new_compiler: Compiler = Default::default();
    println!("{:?}", new_compiler);
    new_compiler.compile("assets/src/test.cpp", "cpp");
}
