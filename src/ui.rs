use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
};
use crate::{App, TimerState};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // 4 vertical bands: header / timer / gauge / footer
    let chunks = Layout::vertical([
        Constraint::Length(3),  // header
        Constraint::Min(7),     // big timer panel
        Constraint::Length(3),  // gauge
        Constraint::Length(2),  // footer
    ])
    .split(area);

    // ---------- Header ----------
    let paused = matches!(app.timer_state, TimerState::Paused { .. });

    let header_line = Line::from(vec![
        Span::from(" Opomodoro ").bold(),
        Span::from(format!(" Cycle {}/{} ", app.current_cycle, app.num_cycles)).bold(),
        Span::from(" ").into(),
        Span::from(app.phase.kind).bold(),
        if paused { Span::from(" (Paused)").bold() } else { Span::from("") },
        if app.late { Span::from("  w/ last break").bold() } else { Span::from("") },
    ]);

    let header = Paragraph::new(header_line)
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .alignment(Alignment::Center);

    frame.render_widget(header, chunks[0]);

    // ---------- Timer panel ----------
    let secs = app.remaining.as_secs();
    let mm = secs / 60;
    let ss = secs % 60;
    let time_str = format!("{:02}:{:02}", mm, ss);

    let timer_block = Block::default().borders(Borders::ALL).title("Timer");
    let inner = timer_block.inner(chunks[1]);

    // Only use big digits if we have enough vertical space inside the block.
    let timer = if inner.height >= BIG_HEIGHT {
        let lines = big_time_lines(mm as u8, ss as u8, inner.height);
        Paragraph::new(lines)
            .block(timer_block)
            .alignment(Alignment::Center)
    } else {
        Paragraph::new(Line::from(time_str).bold())
            .block(timer_block)
            .alignment(Alignment::Center)
    };

    frame.render_widget(timer, chunks[1]);

    // ---------- Gauge ----------
    let total = app.phase.duration;
    let total_s = total.as_secs_f64();
    let rem_s = app.remaining.as_secs_f64();
    let elapsed_s = (total_s - rem_s).max(0.0);
    let ratio = if total_s > 0.0 { elapsed_s / total_s } else { 0.0 };

    let gauge_label = format!(
        "{} / {}",
        format_mmss(elapsed_s as u64),
        format_mmss(total.as_secs())
    );

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL).title("Progress"))
        .ratio(ratio)
        .label(gauge_label);

    frame.render_widget(gauge, chunks[2]);

    // ---------- Footer ----------
    let footer_line = Line::from(vec![
        Span::from(" p ").bold(),
        Span::from("pause/resume   "),
        Span::from(" s ").bold(),
        Span::from("skip   "),
        Span::from(" q ").bold(),
        Span::from("quit"),
    ]);

    let footer = Paragraph::new(footer_line)
        .alignment(Alignment::Center);

    frame.render_widget(footer, chunks[3]);
}

// small helper: render seconds as MM:SS
fn format_mmss(total_secs: u64) -> String {
    let mm = total_secs / 60;
    let ss = total_secs % 60;
    format!("{:02}:{:02}", mm, ss)
}

const BIG_HEIGHT: u16 = 5;

const DIGITS: [[&str; 5]; 10] = [
    // 0
    ["█████", "█   █", "█   █", "█   █", "█████"],
    // 1
    ["  █  ", " ██  ", "  █  ", "  █  ", "█████"],
    // 2
    ["█████", "    █", "█████", "█    ", "█████"],
    // 3
    ["█████", "    █", "█████", "    █", "█████"],
    // 4
    ["█   █", "█   █", "█████", "    █", "    █"],
    // 5
    ["█████", "█    ", "█████", "    █", "█████"],
    // 6
    ["█████", "█    ", "█████", "█   █", "█████"],
    // 7
    ["█████", "    █", "   █ ", "  █  ", "  █  "],
    // 8
    ["█████", "█   █", "█████", "█   █", "█████"],
    // 9
    ["█████", "█   █", "█████", "    █", "█████"],
];

const COLON: [&str; 5] = [
    "  ",
    "██",
    "  ",
    "██",
    "  ",
];

fn glyph(ch: char) -> [&'static str; 5] {
    match ch {
        '0'..='9' => DIGITS[(ch as u8 - b'0') as usize],
        ':' => COLON,
        _ => ["", "", "", "", ""],
    }
}

/// Build big-digit lines for `MM:SS`, vertically centered within `inner_height`.
fn big_time_lines(mm: u8, ss: u8, inner_height: u16) -> Vec<Line<'static>> {
    let text = format!("{:02}:{:02}", mm, ss);
    let chars: Vec<char> = text.chars().collect();

    // First build the 5 content lines (strings).
    let mut content: Vec<String> = Vec::with_capacity(5);
    for row in 0..5 {
        let mut line = String::new();
        for (i, ch) in chars.iter().enumerate() {
            let g = glyph(*ch);
            line.push_str(g[row]);
            if i + 1 < chars.len() {
                line.push(' '); // spacing between glyphs
            }
        }
        content.push(line);
    }

    // Vertically center within the block's inner rect.
    let content_h = BIG_HEIGHT;
    let pad_top = inner_height.saturating_sub(content_h) / 2;

    let mut lines: Vec<Line<'static>> = Vec::new();
    for _ in 0..pad_top {
        lines.push(Line::from("")); // blank padding
    }

    for s in content {
        // Bold the big digits for extra pop.
        lines.push(Line::from(Span::raw(s).bold()));
    }

    lines
}
