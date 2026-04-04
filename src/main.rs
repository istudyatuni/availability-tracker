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

    /// Timeout in seconds to wait for curl
    #[arg(long = "curl", default_value_t = 2)]
    curl_timeout: u64,

    /// Timeout in seconds between requests
    #[arg(long = "sleep", default_value_t = 8)]
    sleep_timeout: u64,

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
    let mut prev_msg = None;
    loop {
        if let Some(msg) = &prev_msg {
            print!("\r{msg} (checking)");
            flush();
        }

        let success = if args.test {
            std::thread::sleep(Duration::from_secs(args.curl_timeout));
            test_rng.next().unwrap() == 0
        } else {
            check(&args.address, args.curl_timeout)
        };
        if let Some(prev_success) = prev_success {
            if success != prev_success {
                if let Some(msg) = prev_msg {
                    print!("\r{msg}{}", " ".repeat(15));
                }
                println!();
            } else {
                print!("\r");
            }
        }
        prev_success = Some(success);
        let time = get_time();
        let msg = if success {
            format!("{ok}     at {time}", ok = "ok".green())
        } else {
            format!("{failed} at {time}", failed = "failed".red())
        };
        for i in 0..args.sleep_timeout {
            if i != 0 {
                print!("\r");
            }
            let i = args.sleep_timeout - i;
            print!("{msg} ({i} s) {}", " ".repeat(4));
            flush();
            std::thread::sleep(Duration::from_secs(1));
        }
        prev_msg = Some(msg);
    }
}

fn check(address: &str, timeout_sec: u64) -> bool {
    let mut handle = Command::new("curl")
        .args(["--silent", address])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn curl command");

    for _ in 0..timeout_sec {
        std::thread::sleep(Duration::from_secs(1));
        if let Some(status) = handle
            .try_wait()
            .expect("failed to wait curl subprocess status")
        {
            return status.success();
        }
    }

    handle.kill().expect("failed to kill curl");

    false
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

fn flush() {
    std::io::stdout().flush().expect("failed to flush stdout");
}
