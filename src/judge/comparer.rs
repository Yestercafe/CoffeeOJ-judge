use std::{
    fs::File,
    io::{BufRead, BufReader},
};

pub struct Comparer {
    lhs_file: Result<File, std::io::Error>,
    rhs_file: Result<File, std::io::Error>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ComparerResult {
    Consistent,
    Inconsistent(usize, String, String),
}

#[derive(Debug)]
pub enum Error {
    FileSystemError,
    IOError,
}

impl Comparer {
    pub fn new(lhs_path: &str, rhs_path: &str) -> Comparer {
        let lhs_file = File::open(lhs_path);
        let rhs_file = File::open(rhs_path);
        Comparer { lhs_file, rhs_file }
    }

    pub fn compare(self) -> Result<ComparerResult, Error> {
        let lhs_file = self.lhs_file.map_err(|_| Error::FileSystemError)?;
        let rhs_file = self.rhs_file.map_err(|_| Error::FileSystemError)?;

        let mut left_reader = BufReader::new(lhs_file);
        let mut right_reader = BufReader::new(rhs_file);
        let mut left_line = String::new();
        let mut right_line = String::new();

        let mut cnt_line = 1;
        loop {
            let left_reader_ret = left_reader.read_line(&mut left_line);
            let right_reader_ret = right_reader.read_line(&mut right_line);

            if left_reader_ret.is_err() || right_reader_ret.is_err() {
                return Err(Error::IOError);
            }
            let mut left_reader_ret = left_reader_ret.unwrap();
            let mut right_reader_ret = right_reader_ret.unwrap();

            if left_reader_ret == 0 || right_reader_ret == 0 {
                if left_reader_ret == 0 && right_reader_ret == 0 {
                    break;
                }
                let mut next_line = String::new();
                if right_reader_ret == 0 {
                    // left != 0, right == 0
                    std::mem::swap(&mut left_reader_ret, &mut right_reader_ret);
                    std::mem::swap(&mut left_line, &mut right_line);
                }
                // always make left == 0, right != 0
                let right_line_trimmed = right_line.trim_end();
                if right_line_trimmed.is_empty()
                    && right_reader.read_line(&mut next_line).unwrap_or(1) == 0
                {
                    break;
                } else {
                    return Ok(ComparerResult::Inconsistent(
                        cnt_line,
                        String::new(),
                        String::new(),
                    ));
                }
            }

            let left_line_trimmed = left_line.trim_end_matches('\n');
            let right_line_trimmed = left_line.trim_end_matches('\n');
            if left_line_trimmed != right_line_trimmed {
                return Ok(ComparerResult::Inconsistent(
                    cnt_line,
                    left_line_trimmed.to_string(),
                    right_line_trimmed.to_string(),
                ));
            }

            cnt_line += 1;
        }

        Ok(ComparerResult::Consistent)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    use crate::judge::comparer::ComparerResult;

    use super::Comparer;

    #[test]
    fn test_bufreader_behavior() {
        let a_file = File::open("assets/tests/comparer/hello_world.txt").unwrap();
        let mut reader = BufReader::new(a_file);
        let mut s = String::new();
        let mut result: Vec<usize> = vec![];

        loop {
            let ret = reader.read_line(&mut s).unwrap();
            if ret == 0 {
                break;
            }
            result.push(ret);
        }

        assert_eq!(result, vec![6, 1, 6, 1]);
    }

    #[test]
    fn test_samples_with_trimmed_blankline() {
        let defense = "assets/tests/comparer/defense.txt";
        // attack 1 and 2 should pass
        for i in 1..=2 {
            let attack = format!("assets/tests/comparer/attack{i}.txt");
            let result = Comparer::new(defense, &attack).compare().unwrap();
            assert_eq!(ComparerResult::Consistent, result);
        }
        // attack 3~6 cannot pass
        for i in 3..=6 {
            let attack = format!("assets/tests/comparer/attack{i}.txt");
            let result = Comparer::new(defense, &attack).compare().unwrap();
            assert_ne!(ComparerResult::Consistent, result);
        }
    }
}
