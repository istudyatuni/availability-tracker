use std::{
    io::Write,
    process::{Command, Stdio},
    sync::LazyLock,
    time::Duration,
};

use clap::Parser;
use owo_colors::OwoColorize;
use time::{UtcOffset, format_description::StaticFormatDescription};

#[derive(Debug, Parser)]
struct Args {
    /// Address to check
    address: String,

    /// Do not perform real requests
    #[arg(long)]
    test: bool,
}

fn main() {
    let args = Args::parse();

    let mut test_rng = [0, 1, 1, 0, 0, 0, 0, 1, 1, 1].into_iter();

    if args.test {
        println!("running in test mode");
    }
    println!("{} at {}", "status".dimmed(), "[last seen]".dimmed());
    println!("start  at {}", get_time());

    let mut prev_success = None;
    loop {
        let success = if args.test {
            test_rng.next().unwrap() == 0
        } else {
            check(&args.address, args.curl_timeout)
        };
        if let Some(prev_success) = prev_success {
            if success != prev_success {
                println!();
            } else {
                print!("\r");
            }
        }
        prev_success = Some(success);
        let time = get_time();
        if success {
            print!("{}     at {time}", "ok".green());
        } else {
            print!("{} at {time}", "failed".red());
        }
        std::io::stdout().flush().expect("failed to flush stdout");
        std::thread::sleep(Duration::from_secs(10));
    }
}

fn check(address: &str) -> bool {
    Command::new("curl")
        .args(["--connect-timeout", "2", "-s", address])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to run curl")
        .success()
}

fn get_time() -> String {
    const FORMAT: StaticFormatDescription = time::macros::format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second]+[offset_hour]:[offset_minute]"
    );

    static OFFSET: LazyLock<UtcOffset> = LazyLock::new(|| {
        UtcOffset::current_local_offset().expect("failed to get local time offset")
    });

    time::UtcDateTime::now()
        .to_offset(*OFFSET)
        .format(&FORMAT)
        .expect("failed to format time")
}
