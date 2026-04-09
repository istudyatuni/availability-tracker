use std::fmt::{Display, write};

use crossterm::style::Stylize;

use crate::FAILED;

#[derive(Debug, Default)]
pub struct Stat {
    fail: u64,
    total: u64,
}

impl Stat {
    pub fn add_success(&mut self, success: bool) {
        if !success {
            self.fail += 1;
        }
        self.total += 1;
    }
    pub fn has_failed(&self) -> bool {
        self.fail > 0
    }
}

impl Display for Stat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let failed = self.fail as f64 / self.total as f64 * 100.0;
        let precision = if failed > 0.1 {
            2
        } else if self.fail == 0 {
            0
        } else {
            5
        };
        write(f, format_args!("{} {failed:.precision$}%", FAILED.red()))
    }
}
