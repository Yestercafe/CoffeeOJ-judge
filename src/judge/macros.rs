#[macro_export]
macro_rules! c_string {
    ($string:expr) => {
        CString::new($string).unwrap()
    };
}

#[macro_export]
macro_rules! c_string_ptr {
    ($string:expr) => {
        c_string!($string).as_ptr()
    };
}
