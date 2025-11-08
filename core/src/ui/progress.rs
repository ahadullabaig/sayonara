use std::io::{self, Write};
use std::time::{Instant};

pub(crate) const CAT_FRAMES: [&str; 6] = [
    "ฅ(^･ω･^=)  ", // cat happy
    "ฅ(=^･ω･^ ) ",
    "ฅ(^･ᴥ･^=)  ",
    "ฅ(=^ᴥ^= )  ",
    "ฅ(^･ω･^=)  ",
    "ฅ(=^･ω･^ ) ",
];

pub(crate) const PAW_FRAMES: [&str; 4] = ["·", "˚", "•", "˚"];

pub struct ProgressBar {
    width: usize,
    cat_pos: usize,
    cat_frame: usize,
    paw_frame: usize,
    start: Instant,
    first_render: bool,
}

impl ProgressBar {
    /// width = number of bar character slots (not including the brackets)
    pub fn new(width: usize) -> Self {
        Self {
            width,
            cat_pos: 0,
            cat_frame: 0,
            paw_frame: 0,
            start: Instant::now(),
            first_render: true,
        }
    }

    /// Render the progress bar
    /// - `progress`: 0.0..=100.0
    /// - `bytes_written` and `total_bytes` are optional. If provided ETA and speed will be shown.
    pub fn render(&mut self, progress: f64, bytes_written: Option<u64>, total_bytes: Option<u64>) {
        // clamp progress
        let pct = if progress.is_nan() {
            0.0
        } else {
            progress.clamp(0.0, 100.0)
        };

        let filled = ((pct / 100.0) * self.width as f64).round() as usize;
        let empty = self.width.saturating_sub(filled);

        // advance animation frames
        self.cat_pos = (self.cat_pos + 1) % (self.width.max(1));
        self.cat_frame = (self.cat_frame + 1) % CAT_FRAMES.len();
        self.paw_frame = (self.paw_frame + 1) % PAW_FRAMES.len();

        // Build cat line (above the bar)
        // cat walks independently across the width, wraps around
        let mut cat_line = vec![' '; self.width + 2]; // include bracket space
        let cat = CAT_FRAMES[self.cat_frame];
        // place cat; ensure it fits
        let cat_chars: Vec<char> = cat.chars().collect();
        let pos = self.cat_pos.min(self.width + 1 - cat_chars.len().max(1));
        for (i, c) in cat_chars.iter().enumerate() {
            if pos + i < cat_line.len() {
                cat_line[pos + i] = *c;
            }
        }
        let cat_line_str: String = cat_line.into_iter().collect();

        // Colors (ANSI) — subtle & modern
        // green for filled, gray for empty, cyan for percent label
        let green = "\x1b[38;5;82m";   // bright green
        let gray = "\x1b[38;5;240m";   // gray
        let cyan = "\x1b[38;5;51m";    // cyan
        let bold = "\x1b[1m";
        let reset = "\x1b[0m";

        // Build bar using block characters
        let filled_block = "█";
        let empty_block = "░";

        let bar_filled = filled_block.repeat(filled);
        let bar_empty = empty_block.repeat(empty);
        let bar = format!(
            "{}{}{}{}{}",
            bold, green, bar_filled, reset, gray
        ) + &bar_empty + reset;

        // Speed and ETA
        let _info = String::new();

        let info = if let (Some(written), Some(total)) = (bytes_written, total_bytes) {
            let elapsed = self.start.elapsed().as_secs_f64().max(0.0001);
            let speed = (written as f64) / elapsed;
            let speed_readable = human_bytes(speed);
            let remaining = if total > written { total - written } else { 0 };
            let eta_secs = if speed > 0.0 {
                (remaining as f64 / speed).round() as u64
            } else {
                0
            };
            let eta = format_duration(eta_secs);

            format!(
                "{}{:.1}%{}  {} @ {}/s  ETA {}",
                bold, pct, reset, cyan, speed_readable, eta
            )
        } else {
            let paw = PAW_FRAMES[self.paw_frame];
            format!("{}{:.1}%{}  {}working...{}", bold, pct, reset, cyan, paw)
        };

        // Clear previous lines if we've printed before
        if self.first_render {
            // print two lines (cat + bar+info)
            print!("{}\n[{}] {}\n", cat_line_str, bar, info);
            self.first_render = false;
        } else {
            // move cursor up 2 lines, clear them, reprint
            // \x1b[2A moves up 2 lines, \x1b[2K clears line
            print!("\x1b[2A\x1b[2K\r"); // go up 2 and clear
            print!("{}\n", cat_line_str);
            print!("\x1b[2K\r[{}] {}\n", bar, info);
        }

        io::stdout().flush().ok();
    }
}

/// Convert bytes/sec to readable string
pub(crate) fn human_bytes(bps: f64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    if bps <= 0.0 {
        return "0B".to_string();
    }
    let mut val = bps;
    let mut i = 0usize;
    while val >= 1024.0 && i + 1 < units.len() {
        val /= 1024.0;
        i += 1;
    }
    format!("{:.2}{}", val, units[i])
}

/// Format seconds to H:MM:SS or M:SS
pub(crate) fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}:{:02}:{:02}", h, m, s)
    } else {
        format!("{}:{:02}", m, s)
    }
}
