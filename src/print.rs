//! Pretty server-side rendering of messages as they pass through the hub.
//!
//! Each message is printed with an aligned header — local time, id, sender, an
//! arrow, recipient — followed by the body. When stderr is a terminal the body
//! is word-wrapped to the terminal width and continuation lines are indented to
//! line up under the body column. Every agent name gets a stable colour so the
//! same agent always looks the same in the feed. Colours are disabled
//! automatically when stderr isn't a terminal or when `NO_COLOR` is set.

use std::io::IsTerminal;

use chrono::{Local, TimeZone};

use crate::hub::{Message, BROADCAST};

/// Width the agent-name columns are padded to, for alignment.
const NAME_W: usize = 12;
/// Visible width of the header before the body starts — this is also the indent
/// continuation lines hang at. Keep in sync with the format strings below:
/// `HH:MM:SS`(8) + `  `(2) + `#`(1) + id(4) + ` `(1) + from(12) + ` `(1) +
/// arrow(3) + ` `(1) + to(12) + `  `(2) = 47.
const PREFIX_W: usize = 47;
/// Never wrap the body narrower than this, even on a tiny terminal.
const MIN_BODY: usize = 24;

/// 256-colour palette — readable, distinct hues on a dark background.
const PALETTE: [u8; 6] = [39, 78, 214, 170, 203, 156];

fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

/// Body column width to wrap to, or `None` when stderr isn't a terminal (then
/// the body is printed on a single line, unwrapped).
fn wrap_width() -> Option<usize> {
    if !std::io::stderr().is_terminal() {
        return None;
    }
    terminal_size::terminal_size()
        .map(|(w, _)| (w.0 as usize).saturating_sub(PREFIX_W).max(MIN_BODY))
}

/// Deterministic colour for an agent name, so each agent reads consistently.
fn color_for(name: &str) -> u8 {
    let sum: u32 = name.bytes().map(u32::from).sum();
    PALETTE[(sum as usize) % PALETTE.len()]
}

/// Pad (or truncate) `s` to exactly `w` display columns. Names are short and
/// effectively single-width, so counting chars is good enough here.
fn pad(s: &str, w: usize) -> String {
    let n = s.chars().count();
    if n >= w {
        s.chars().take(w).collect()
    } else {
        format!("{s}{}", " ".repeat(w - n))
    }
}

fn body_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Word-wrap `text` to `width` columns, hard-breaking any single word (e.g. a
/// long JSON blob with no spaces) that doesn't fit on its own.
fn wrap_line(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut cur = String::new();
    let mut cur_len = 0usize;

    for word in text.split(' ') {
        let wlen = word.chars().count();
        if wlen > width {
            if cur_len > 0 {
                lines.push(std::mem::take(&mut cur));
            }
            let mut chunk = String::new();
            let mut clen = 0usize;
            for ch in word.chars() {
                if clen == width {
                    lines.push(std::mem::take(&mut chunk));
                    clen = 0;
                }
                chunk.push(ch);
                clen += 1;
            }
            cur = chunk;
            cur_len = clen;
            continue;
        }
        let added = if cur_len == 0 { wlen } else { cur_len + 1 + wlen };
        if added > width {
            lines.push(std::mem::take(&mut cur));
            cur.push_str(word);
            cur_len = wlen;
        } else {
            if cur_len > 0 {
                cur.push(' ');
                cur_len += 1;
            }
            cur.push_str(word);
            cur_len += wlen;
        }
    }
    if cur_len > 0 || lines.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Split a body into the lines to print. With a wrap width, honours embedded
/// newlines as hard breaks and wraps each paragraph; without one, collapses to
/// a single line.
fn body_lines(v: &serde_json::Value, wrap: Option<usize>) -> Vec<String> {
    let raw = body_text(v);
    match wrap {
        None => vec![raw.replace('\n', "⏎")],
        Some(width) => {
            let mut out = Vec::new();
            for para in raw.split('\n') {
                out.extend(wrap_line(para, width));
            }
            if out.is_empty() {
                out.push(String::new());
            }
            out
        }
    }
}

fn hms(ts: u64) -> String {
    Local
        .timestamp_millis_opt(ts as i64)
        .single()
        .map(|d| d.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| "--:--:--".to_string())
}

/// Print one message to stderr, the way the running hub shows its live feed.
pub fn message(msg: &Message) {
    let time = hms(msg.ts);
    let to_disp = if msg.to == BROADCAST { "all" } else { &msg.to };
    let from = pad(&msg.from, NAME_W);
    let to = pad(to_disp, NAME_W);
    let lines = body_lines(&msg.body, wrap_width());
    let indent = " ".repeat(PREFIX_W);

    if !colors_enabled() {
        eprintln!("{time}  #{:<4} {from} --> {to}  {}", msg.id, lines[0]);
        for cont in &lines[1..] {
            eprintln!("{indent}{cont}");
        }
        return;
    }

    const DIM: &str = "\x1b[2m";
    const BOLD: &str = "\x1b[1m";
    const RESET: &str = "\x1b[0m";
    let fg = |c: u8| format!("\x1b[38;5;{c}m");

    let from_c = fg(color_for(&msg.from));
    // Broadcasts get a neutral grey recipient; everyone else their own colour.
    let to_c = if msg.to == BROADCAST {
        fg(245)
    } else {
        fg(color_for(&msg.to))
    };

    eprintln!(
        "{DIM}{time}{RESET}  {DIM}#{id:<4}{RESET} {BOLD}{from_c}{from}{RESET} {DIM}──▶{RESET} {to_c}{to}{RESET}  {body}",
        id = msg.id,
        body = lines[0],
    );
    for cont in &lines[1..] {
        eprintln!("{indent}{cont}");
    }
}
