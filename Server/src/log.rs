use std::sync::OnceLock;
use std::time::Instant;

/// Seconds elapsed since the first call (first call happens at the top of main).
pub fn elapsed() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}

/// Name of the OS thread running the caller, e.g. "interface-worker-3" or "dispatcher".
pub fn thread_name() -> String {
    std::thread::current().name().unwrap_or("?").to_string()
}

/// Prefixes every line with the time since startup and the current thread name.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        println!(
            "[{:>9.3}s] [{:<18}] {}",
            $crate::log::elapsed(),
            $crate::log::thread_name(),
            format!($($arg)*)
        )
    };
}

/// Renders bytes as "00 00 00 00 00 00 00 2a 01".
pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
