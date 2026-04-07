use std::{
    io::Write,
    process::{Command, Stdio},
    sync::LazyLock,
    time::{Duration, Instant},
};

use clap::Parser;
use crossterm::{ExecutableCommand, cursor};
use owo_colors::OwoColorize;
use signal_hook::{consts::SIGINT, iterator::Signals};
use time::{UtcOffset, format_description::StaticFormatDescription};

const STATUS: &str = "status";
const OK: &str = "ok";
const FAILED: &str = "failed";

/// Max len of `["status", "ok", "failed"]`
///
/// Note that if it's required to align string after using `to_string()`, e.g.:
///
/// ```
/// let ok = OK.green().to_string();
/// println!("{ok:<PREFIX_LEN$}");
/// ```
///
/// prefix should be adjusted to include number of additional symbols for
/// ascii color code, e.g. for fg colors:
///
/// ```
/// let ok = OK.green().to_string();
/// println!("{ok:<width$}", width = PREFIX_LEN + 10);
/// ```
///
/// 10 is because green looks like `\[32mok\[39m`
const PREFIX_LEN: usize = 6;

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
    #[arg(long, hide = true)]
    test: bool,
}

fn main() {
    let args = Args::parse();

    hide_cursor().unwrap();
    install_panic_hook();
    install_signal_handler().unwrap();

    let mut test_rng = [0, 1, 1, 0, 0, 0, 0, 1, 1, 1].into_iter();

    if args.test {
        println!("running in test mode");
    }
    println!(
        "{:<PREFIX_LEN$} at {:<seen_width$} (unchanged for {})",
        STATUS.dimmed(),
        "first seen".dimmed(),
        "how long".dimmed(),
        seen_width = get_time().len(),
    );

    let mut prev_success = None;
    let mut timer = None;
    loop {
        print!("checking{:<10}\r", "");
        flush();

        let success = if args.test {
            std::thread::sleep(Duration::from_secs(args.curl_timeout));
            test_rng.next().unwrap() == 0
        } else {
            check(&args.address, args.curl_timeout, timer)
        };

        if prev_success.is_none_or(|prev| prev != success) {
            println!("{}", format_msg(success));
            timer = Some(Instant::now());
        }
        prev_success = Some(success);
        for i in 0..args.sleep_timeout {
            print_append_how_long(timer).unwrap();

            let i = args.sleep_timeout - i;
            print!("{i}s{:<10}\r", "");
            flush();
            std::thread::sleep(Duration::from_secs(1));
        }
    }
}

fn check(address: &str, timeout_sec: u64, cur_timer: Option<Instant>) -> bool {
    let mut handle = Command::new("curl")
        .args(["--silent", address])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn curl command");

    for _ in 0..timeout_sec {
        print_append_how_long(cur_timer).unwrap();

        std::thread::sleep(Duration::from_secs(1));
        if let Some(status) = handle
            .try_wait()
            .expect("failed to wait curl subprocess status")
        {
            return status.success();
        }
    }

    handle.kill().expect("failed to kill curl");
    // cleanup process so it won't become zombie
    let _ = handle.wait();

    false
}

fn get_time() -> String {
    const FORMAT: StaticFormatDescription =
        time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");

    static OFFSET: LazyLock<UtcOffset> =
        LazyLock::new(|| UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC));

    time::UtcDateTime::now()
        .to_offset(*OFFSET)
        .format(&FORMAT)
        .expect("failed to format time")
}

fn print_append_how_long(timer: Option<Instant>) -> Result<(), std::io::Error> {
    static LEN: LazyLock<u16> =
        LazyLock::new(|| (PREFIX_LEN + " at ".len() + get_time().len()) as u16);

    let Some(timer) = timer else {
        return Ok(());
    };

    std::io::stdout()
        .execute(cursor::MoveToColumn(0))?
        .execute(cursor::MoveUp(1))?
        .execute(cursor::MoveRight(*LEN))?;
    println!(" (for {}){:<10}", format_sec(timer.elapsed()), "");
    Ok(())
}

fn show_cursor() -> Result<(), std::io::Error> {
    std::io::stdout().execute(cursor::Show)?;
    Ok(())
}

fn hide_cursor() -> Result<(), std::io::Error> {
    std::io::stdout().execute(cursor::Hide)?;
    Ok(())
}

fn install_signal_handler() -> Result<(), Box<dyn std::error::Error>> {
    let mut signals = Signals::new([SIGINT])?;

    std::thread::spawn(move || {
        signals.forever().next().unwrap();
        show_cursor().unwrap();
        println!();
        std::process::exit(0);
    });

    Ok(())
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Err(e) = show_cursor() {
            eprintln!("failed to restore cursor: {e}");
        }
        original_hook(info);
    }));
}

fn format_msg(success: bool) -> String {
    let time = get_time();
    let status = if success {
        OK.green().into_styled()
    } else {
        FAILED.red().into_styled()
    };
    format!("{status:<PREFIX_LEN$} at {time}")
}

fn format_sec(dur: Duration) -> String {
    // remove precision higher than sec
    let elapsed = Duration::from_secs(dur.as_secs());
    humantime::format_duration(elapsed).to_string()
}

fn flush() {
    std::io::stdout().flush().expect("failed to flush stdout");
}
