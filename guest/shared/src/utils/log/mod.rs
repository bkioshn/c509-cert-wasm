//! Log utilities.

/// Print out the message.
pub fn log(msg: &str) {
    println!("{msg}");
}

/// Append the line to the file.
pub fn append(path: &str, line: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut f = OpenOptions::new()
        .create(true).append(true)
        .open(path).expect("open log");
    f.write_all(line.as_bytes()).expect("write log");
}