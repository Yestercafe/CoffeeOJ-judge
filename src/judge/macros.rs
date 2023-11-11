#[macro_export]
macro_rules! c_string {
    ($string:expr) => {
        CString::new($string).unwrap() // TODO handle CString construction error
    };
}
