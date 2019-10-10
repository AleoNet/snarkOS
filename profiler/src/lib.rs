#![allow(unused_imports)]
pub use self::inner::*;

#[cfg(feature = "print-trace")]
#[macro_use]
pub mod inner {
    use std::{sync::atomic::AtomicUsize, time::Instant};

    pub use colored::Colorize;

    pub static NUM_INDENT: AtomicUsize = AtomicUsize::new(0);
    pub const PAD_CHAR: &'static str = "·";

    pub struct TimerInfo {
        pub msg: String,
        pub time: Instant,
    }

    #[macro_export]
    macro_rules! start_timer {
        ($msg:expr) => {{
            use std::{sync::atomic::Ordering, time::Instant};
            use $crate::{compute_indent, Colorize, NUM_INDENT, PAD_CHAR};

            let msg = $msg();
            let start_info = "Start:".yellow().bold();
            let indent_amount = 2 * NUM_INDENT.fetch_add(0, Ordering::Relaxed);
            let indent = compute_indent(indent_amount);

            println!("{}{:8} {}", indent, start_info, msg);
            NUM_INDENT.fetch_add(1, Ordering::Relaxed);
            $crate::TimerInfo {
                msg: msg.to_string(),
                time: Instant::now(),
            }
        }};
    }

    #[macro_export]
    macro_rules! end_timer {
        ($time:expr) => {{
            end_timer!($time, || "");
        }};
        ($time:expr, $msg:expr) => {{
            use std::sync::atomic::Ordering;
            use $crate::{compute_indent, Colorize, NUM_INDENT, PAD_CHAR};

            let time = $time.time;
            let final_time = time.elapsed();
            let final_time = {
                let secs = final_time.as_secs();
                let millis = final_time.subsec_millis();
                let micros = final_time.subsec_micros() % 1000;
                let nanos = final_time.subsec_nanos() % 1000;
                if secs != 0 {
                    format!("{}.{}s", secs, millis).bold()
                } else if millis > 0 {
                    format!("{}.{}ms", millis, micros).bold()
                } else if micros > 0 {
                    format!("{}.{}µs", micros, nanos).bold()
                } else {
                    format!("{}ns", final_time.subsec_nanos()).bold()
                }
            };

            let end_info = "End:".green().bold();
            let message = format!("{} {}", $time.msg, $msg());

            NUM_INDENT.fetch_sub(1, Ordering::Relaxed);
            let indent_amount = 2 * NUM_INDENT.fetch_add(0, Ordering::Relaxed);
            let indent = compute_indent(indent_amount);

            // Todo: Recursively ensure that *entire* string is of appropriate
            // width (not just message).
            println!(
                "{}{:8} {:.<pad$}{}",
                indent,
                end_info,
                message,
                final_time,
                pad = 75 - indent_amount
            );
        }};
    }

    #[macro_export]
    macro_rules! add_to_trace {
        ($title:expr, $msg:expr) => {{
            use std::sync::atomic::Ordering;
            use $crate::{compute_indent, compute_indent_whitespace, Colorize, NUM_INDENT, PAD_CHAR};

            let start_msg = "StartMsg".yellow().bold();
            let end_msg = "EndMsg".green().bold();
            let title = $title();
            let start_msg = format!("{}: {}", start_msg, title);
            let end_msg = format!("{}: {}", end_msg, title);

            let start_indent_amount = 2 * NUM_INDENT.fetch_add(0, Ordering::Relaxed);
            let start_indent = compute_indent(start_indent_amount);

            let msg_indent_amount = 2 * NUM_INDENT.fetch_add(0, Ordering::Relaxed) + 2;
            let msg_indent = compute_indent_whitespace(msg_indent_amount);
            let mut final_message = "\n".to_string();
            for line in $msg().lines() {
                final_message += &format!("{}{}\n", msg_indent, line,);
            }

            // Todo: Recursively ensure that *entire* string is of appropriate
            // width (not just message).
            println!("{}{}", start_indent, start_msg);
            println!("{}{}", msg_indent, final_message,);
            println!("{}{}", start_indent, end_msg);
        }};
    }

    pub fn compute_indent_whitespace(indent_amount: usize) -> String {
        let mut indent = String::new();
        for _ in 0..indent_amount {
            indent.push_str(" ");
        }
        indent
    }

    pub fn compute_indent(indent_amount: usize) -> String {
        use std::env::var;
        let mut indent = String::new();
        let pad_string = match var("CLICOLOR") {
            Ok(val) => {
                if val == "0" {
                    " "
                } else {
                    PAD_CHAR
                }
            }
            Err(_) => PAD_CHAR,
        };
        for _ in 0..indent_amount {
            indent.push_str(&pad_string.white());
        }
        indent
    }
}

#[cfg(not(feature = "print-trace"))]
#[macro_use]
mod inner {
    pub struct TimerInfo;

    #[macro_export]
    macro_rules! start_timer {
        ($msg:expr) => {
            $crate::TimerInfo
        };
    }
    #[macro_export]
    macro_rules! add_to_trace {
        ($title:expr, $msg:expr) => {
            let _ = $msg;
        };
    }

    #[macro_export]
    macro_rules! end_timer {
        ($time:expr, $msg:expr) => {
            let _ = $msg;
            let _ = $time;
        };
        ($time:expr) => {
            let _ = $time;
        };
    }
}

mod tests {
    use super::*;

    #[test]
    fn print_start_end() {
        let start = start_timer!(|| "Hello");
        end_timer!(start);
    }

    #[test]
    fn print_add() {
        let start = start_timer!(|| "Hello");
        add_to_trace!(|| "HelloMsg", || "Hello, I\nAm\nA\nMessage");
        end_timer!(start);
    }
}
