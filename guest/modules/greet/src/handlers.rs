use shared::utils::{log, timer};

pub fn greet(name: String) {
    let secs = timer::now_secs();
    let line = format!("[{secs}] Hello, {name}!");
    log::log(&line);
    log::append("greetings.log", &format!("{line}\n"));
}