//! Terminal screen state wrapper using the `vt100` crate.
//! Converts a PTY byte stream into a renderable fixed-size cell grid.

use egui::Color32;

/// Text modifiers (bold, italic, etc.)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TextStyle {
    pub bold:          bool,
    pub italic:        bool,
    pub underline:     bool,
    pub dim:           bool,
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
    pub fn plain_text(&self) -> String {
        self.cells.iter().map(|c| c.ch).collect()
    }
}

pub struct AnsiParser {
    parser:             vt100::Parser,
    rows:               u16,
    cols:               u16,
    rendered_lines:     Vec<StyledLine>,
    cursor_position:    Option<(u16, u16)>,
    application_cursor: bool,
}

impl AnsiParser {
    pub fn new(rows: u16, cols: u16, scrollback_len: usize) -> Self {
        let rows = rows.max(2);
        let cols = cols.max(8);
        let parser = vt100::Parser::new(rows, cols, scrollback_len);
        let mut this = Self {
            parser,
            rows,
            cols,
            rendered_lines: Vec::new(),
            cursor_position: None,
            application_cursor: false,
        };
        this.refresh_screen_cache();
        this
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
        self.refresh_screen_cache();
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let rows = rows.max(2);
        let cols = cols.max(8);
        if rows == self.rows && cols == self.cols {
            return;
        }

        self.rows = rows;
        self.cols = cols;
        self.parser.set_size(rows, cols);
        self.refresh_screen_cache();
    }

    pub fn lines(&self) -> &[StyledLine] {
        &self.rendered_lines
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        self.cursor_position
    }

    pub fn application_cursor(&self) -> bool {
        self.application_cursor
    }

    pub fn size(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    fn refresh_screen_cache(&mut self) {
        let screen = self.parser.screen();
        let (cursor_row, cursor_col) = screen.cursor_position();
        self.cursor_position = if screen.hide_cursor() {
            None
        } else {
            Some((cursor_row, cursor_col))
        };
        self.application_cursor = screen.application_cursor();

        let mut lines = Vec::with_capacity(self.rows as usize);
        for row in 0..self.rows {
            let mut cells = Vec::with_capacity(self.cols as usize);
            for col in 0..self.cols {
                let cell = screen.cell(row, col);
                let mut style = cell
                    .map(cell_style)
                    .unwrap_or_default();
                let mut ch = cell_char(cell);

                if Some((row, col)) == self.cursor_position {
                    apply_cursor_style(&mut style, &mut ch);
                }

                cells.push(StyledCell { ch, style });
            }
            lines.push(StyledLine { cells });
        }
        self.rendered_lines = lines;
    }
}

fn cell_char(cell: Option<&vt100::Cell>) -> char {
    let Some(cell) = cell else {
        return ' ';
    };

    let contents = cell.contents();
    if contents.is_empty() {
        ' '
    } else {
        contents.chars().next().unwrap_or(' ')
    }
}

fn cell_style(cell: &vt100::Cell) -> CellStyle {
    let mut style = CellStyle {
        fg: color_to_egui(cell.fgcolor()),
        bg: color_to_egui(cell.bgcolor()),
        text: TextStyle {
            bold: cell.bold(),
            italic: cell.italic(),
            underline: cell.underline(),
            dim: false,
            strikethrough: false,
        },
    };

    if cell.inverse() {
        std::mem::swap(&mut style.fg, &mut style.bg);
    }

    style
}

fn apply_cursor_style(style: &mut CellStyle, ch: &mut char) {
    if *ch == ' ' {
        *ch = ' ';
    }

    let fg = style.fg.unwrap_or(Color32::BLACK);
    let bg = style.bg.unwrap_or(Color32::WHITE);
    style.fg = Some(bg);
    style.bg = Some(fg);
}

fn color_to_egui(color: vt100::Color) -> Option<Color32> {
    match color {
        vt100::Color::Default => None,
        vt100::Color::Idx(idx) => Some(color_index_to_rgb(idx)),
        vt100::Color::Rgb(r, g, b) => Some(Color32::from_rgb(r, g, b)),
    }
}

fn color_index_to_rgb(idx: u8) -> Color32 {
    match idx {
        0 => Color32::from_rgb(0, 0, 0),
        1 => Color32::from_rgb(205, 49, 49),
        2 => Color32::from_rgb(13, 188, 121),
        3 => Color32::from_rgb(229, 229, 16),
        4 => Color32::from_rgb(36, 114, 200),
        5 => Color32::from_rgb(188, 63, 188),
        6 => Color32::from_rgb(17, 168, 205),
        7 => Color32::from_rgb(229, 229, 229),
        8 => Color32::from_rgb(102, 102, 102),
        9 => Color32::from_rgb(241, 76, 76),
        10 => Color32::from_rgb(35, 209, 139),
        11 => Color32::from_rgb(245, 245, 67),
        12 => Color32::from_rgb(59, 142, 234),
        13 => Color32::from_rgb(214, 112, 214),
        14 => Color32::from_rgb(41, 184, 219),
        15 => Color32::from_rgb(255, 255, 255),
        16..=231 => {
            let idx = idx - 16;
            let r = idx / 36;
            let g = (idx % 36) / 6;
            let b = idx % 6;
            Color32::from_rgb(cube_channel(r), cube_channel(g), cube_channel(b))
        }
        232..=255 => {
            let gray = 8u8.saturating_add((idx - 232) * 10);
            Color32::from_rgb(gray, gray, gray)
        }
    }
}

fn cube_channel(value: u8) -> u8 {
    match value {
        0 => 0,
        1 => 95,
        2 => 135,
        3 => 175,
        4 => 215,
        _ => 255,
    }
}
