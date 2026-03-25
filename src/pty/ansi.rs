//! ANSI/VT100 parser wrapper using the `vte` crate.
//! Converts raw bytes from the PTY into a sequence of styled terminal cells.
//! Uses egui color types directly.

use egui::Color32;
use vte::{Params, Parser, Perform};

// ── Style types (egui-native, no ratatui) ────────────────────────────────────

/// Text modifiers (bold, italic, etc.)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextStyle {
    pub bold:      bool,
    pub italic:    bool,
    pub underline: bool,
    pub dim:       bool,
    pub strikethrough: bool,
}

/// Style for a single terminal cell.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CellStyle {
    pub fg:   Option<Color32>,
    pub bg:   Option<Color32>,
    pub text: TextStyle,
}

/// A single styled character cell.
#[derive(Debug, Clone, PartialEq)]
pub struct StyledCell {
    pub ch:    char,
    pub style: CellStyle,
}

/// A line of styled cells.
#[derive(Debug, Clone, Default)]
pub struct StyledLine {
    pub cells: Vec<StyledCell>,
}

impl StyledLine {
    /// Extract plain text (ANSI stripped) for history storage.
    pub fn plain_text(&self) -> String {
        self.cells.iter().map(|c| c.ch).collect()
    }
}

/// ANSI state machine: feeds bytes and produces finished `StyledLine`s.
pub struct AnsiParser {
    parser:    Parser,
    performer: Performer,
}

impl AnsiParser {
    pub fn new() -> Self {
        Self {
            parser:    Parser::new(),
            performer: Performer::new(),
        }
    }

    /// Feed raw bytes into the parser.
    /// Returns any complete lines (terminated by `\n`) produced.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<StyledLine> {
        for &byte in bytes {
            self.parser.advance(&mut self.performer, byte);
        }
        std::mem::take(&mut self.performer.finished_lines)
    }
}

// ── Internal performer ───────────────────────────────────────────────────────

struct Performer {
    current_line:   StyledLine,
    current_style:  CellStyle,
    finished_lines: Vec<StyledLine>,
}

impl Performer {
    fn new() -> Self {
        Self {
            current_line:   StyledLine::default(),
            current_style:  CellStyle::default(),
            finished_lines: Vec::new(),
        }
    }

    fn commit_line(&mut self) {
        let line = std::mem::take(&mut self.current_line);
        self.finished_lines.push(line);
    }
}

impl Perform for Performer {
    fn print(&mut self, c: char) {
        self.current_line.cells.push(StyledCell { ch: c, style: self.current_style });
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.commit_line(),
            b'\r' => {}
            b'\t' => {
                for _ in 0..4 {
                    self.current_line.cells.push(StyledCell { ch: ' ', style: self.current_style });
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        if action == 'm' {
            apply_sgr(&mut self.current_style, params);
        }
    }

    fn hook(&mut self, _: &Params, _: &[u8], _: bool, _: char) {}
    fn put(&mut self, _: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _: &[&[u8]], _: bool) {}
    fn esc_dispatch(&mut self, _: &[u8], _: bool, _: u8) {}
}

// ── SGR attribute parser ─────────────────────────────────────────────────────

fn apply_sgr(style: &mut CellStyle, params: &Params) {
    let mut iter = params.iter();
    while let Some(param) = iter.next() {
        let code = param[0];
        match code {
            0  => *style = CellStyle::default(),
            1  => style.text.bold      = true,
            2  => style.text.dim       = true,
            3  => style.text.italic    = true,
            4  => style.text.underline = true,
            9  => style.text.strikethrough = true,
            22 => { style.text.bold = false; style.text.dim = false; }
            23 => style.text.italic    = false,
            24 => style.text.underline = false,
            // Foreground (30–37)
            30 => style.fg = Some(Color32::from_rgb(30, 30, 30)),
            31 => style.fg = Some(Color32::from_rgb(205, 49,  49)),
            32 => style.fg = Some(Color32::from_rgb(13,  188, 121)),
            33 => style.fg = Some(Color32::from_rgb(229, 229, 16)),
            34 => style.fg = Some(Color32::from_rgb(36,  114, 200)),
            35 => style.fg = Some(Color32::from_rgb(188, 63,  188)),
            36 => style.fg = Some(Color32::from_rgb(17,  168, 205)),
            37 => style.fg = Some(Color32::from_rgb(229, 229, 229)),
            39 => style.fg = None,
            // Background (40–47)
            40 => style.bg = Some(Color32::from_rgb(0,   0,   0)),
            41 => style.bg = Some(Color32::from_rgb(205, 49,  49)),
            42 => style.bg = Some(Color32::from_rgb(13,  188, 121)),
            43 => style.bg = Some(Color32::from_rgb(229, 229, 16)),
            44 => style.bg = Some(Color32::from_rgb(36,  114, 200)),
            45 => style.bg = Some(Color32::from_rgb(188, 63,  188)),
            46 => style.bg = Some(Color32::from_rgb(17,  168, 205)),
            47 => style.bg = Some(Color32::from_rgb(229, 229, 229)),
            49 => style.bg = None,
            // Bright foreground (90–97)
            90 => style.fg = Some(Color32::from_rgb(102, 102, 102)),
            91 => style.fg = Some(Color32::from_rgb(241, 76,  76)),
            92 => style.fg = Some(Color32::from_rgb(35,  209, 139)),
            93 => style.fg = Some(Color32::from_rgb(245, 245, 67)),
            94 => style.fg = Some(Color32::from_rgb(59,  142, 234)),
            95 => style.fg = Some(Color32::from_rgb(214, 112, 214)),
            96 => style.fg = Some(Color32::from_rgb(41,  184, 219)),
            97 => style.fg = Some(Color32::WHITE),
            // 256-color / truecolor — skip for now
            _ => {}
        }
    }
}
