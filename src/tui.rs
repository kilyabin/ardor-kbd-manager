use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap},
};

use crate::device::Device;
use crate::protocol::{Direction, Effect, KeyPosition, Mode, Rgb, custom_reports, key_position};

const EFFECTS: &[(&str, Option<Effect>)] = &[
    ("Off", None),
    ("Static", Some(Effect::Static)),
    ("Breathe", Some(Effect::Breathe)),
    ("Wave", Some(Effect::Wave)),
    ("Neon", Some(Effect::Neon)),
    ("Snake", Some(Effect::Snake)),
    ("Reactive", Some(Effect::Reactive)),
    ("Aurora", Some(Effect::Aurora)),
    ("Ripple", Some(Effect::Ripple)),
    ("Custom", Some(Effect::Custom)),
    ("Music", Some(Effect::Music)),
];

const KEY_ROWS: &[&[(&str, &str, u16)]] = &[
    &[
        ("esc", "Esc", 4),
        ("f1", "F1", 4),
        ("f2", "F2", 4),
        ("f3", "F3", 4),
        ("f4", "F4", 4),
        ("f5", "F5", 4),
        ("f6", "F6", 4),
        ("f7", "F7", 4),
        ("f8", "F8", 4),
        ("f9", "F9", 4),
        ("f10", "F10", 4),
        ("f11", "F11", 4),
        ("f12", "F12", 4),
        ("print", "Prt", 4),
        ("scroll", "Scr", 4),
        ("pause", "Pau", 4),
    ],
    &[
        ("grave", "`", 4),
        ("1", "1", 4),
        ("2", "2", 4),
        ("3", "3", 4),
        ("4", "4", 4),
        ("5", "5", 4),
        ("6", "6", 4),
        ("7", "7", 4),
        ("8", "8", 4),
        ("9", "9", 4),
        ("0", "0", 4),
        ("minus", "-", 4),
        ("equal", "=", 4),
        ("backspace", "Bksp", 6),
        ("insert", "Ins", 4),
        ("home", "Home", 5),
        ("pageup", "PgUp", 5),
    ],
    &[
        ("tab", "Tab", 4),
        ("q", "Q", 4),
        ("w", "W", 4),
        ("e", "E", 4),
        ("r", "R", 4),
        ("t", "T", 4),
        ("y", "Y", 4),
        ("u", "U", 4),
        ("i", "I", 4),
        ("o", "O", 4),
        ("p", "P", 4),
        ("leftbracket", "[", 4),
        ("rightbracket", "]", 4),
        ("backslash", "\\", 4),
        ("delete", "Del", 5),
        ("end", "End", 4),
        ("pagedown", "PgDn", 5),
    ],
    &[
        ("capslock", "Caps", 6),
        ("a", "A", 4),
        ("s", "S", 4),
        ("d", "D", 4),
        ("f", "F", 4),
        ("g", "G", 4),
        ("h", "H", 4),
        ("j", "J", 4),
        ("k", "K", 4),
        ("l", "L", 4),
        ("semicolon", ";", 4),
        ("quote", "'", 4),
        ("enter", "Enter", 6),
    ],
    &[
        ("leftshift", "Shift", 6),
        ("z", "Z", 4),
        ("x", "X", 4),
        ("c", "C", 4),
        ("v", "V", 4),
        ("b", "B", 4),
        ("n", "N", 4),
        ("m", "M", 4),
        ("comma", ",", 4),
        ("period", ".", 4),
        ("slash", "/", 4),
        ("rightshift", "Shift", 6),
        ("up", "↑", 4),
    ],
    &[
        ("leftctrl", "Ctrl", 6),
        ("win", "Win", 4),
        ("leftalt", "Alt", 5),
        ("space", "Space", 12),
        ("rightalt", "Alt", 5),
        ("fn", "Fn", 4),
        ("menu", "Menu", 5),
        ("rightctrl", "Ctrl", 6),
        ("left", "←", 4),
        ("down", "↓", 5),
        ("right", "→", 5),
    ],
];

const MODE_CONTROL_COUNT: usize = 11;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Mode,
    PerKey,
    Help,
}

impl Screen {
    fn index(self) -> usize {
        match self {
            Self::Mode => 0,
            Self::PerKey => 1,
            Self::Help => 2,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModeFocus {
    Effects,
    Controls,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum KeyFocus {
    Grid,
    Red,
    Green,
    Blue,
    Apply,
}

struct ModeState {
    effect_index: usize,
    foreground: Rgb,
    background: Rgb,
    brightness: u8,
    speed: u8,
    direction: Direction,
    full_color: bool,
    focus: ModeFocus,
    control_index: usize,
    apply_requested: bool,
}

impl ModeState {
    fn new() -> Self {
        Self {
            effect_index: 1,
            foreground: Rgb::new(0, 0, 255),
            background: Rgb::OFF,
            brightness: 4,
            speed: 3,
            direction: Direction::Right,
            full_color: false,
            focus: ModeFocus::Effects,
            control_index: 0,
            apply_requested: false,
        }
    }

    fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            ModeFocus::Effects => ModeFocus::Controls,
            ModeFocus::Controls => ModeFocus::Effects,
        };
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let fine = key.modifiers.contains(KeyModifiers::SHIFT);
        match self.focus {
            ModeFocus::Effects => match key.code {
                KeyCode::Up | KeyCode::Left => {
                    self.effect_index = (self.effect_index + EFFECTS.len() - 1) % EFFECTS.len();
                }
                KeyCode::Down | KeyCode::Right => {
                    self.effect_index = (self.effect_index + 1) % EFFECTS.len();
                }
                KeyCode::Enter | KeyCode::Char(' ') => self.apply_requested = true,
                _ => {}
            },
            ModeFocus::Controls => match key.code {
                KeyCode::Up => {
                    self.control_index =
                        (self.control_index + MODE_CONTROL_COUNT - 1) % MODE_CONTROL_COUNT;
                }
                KeyCode::Down => {
                    self.control_index = (self.control_index + 1) % MODE_CONTROL_COUNT;
                }
                KeyCode::Left => self.adjust_control(-1, fine),
                KeyCode::Right => self.adjust_control(1, fine),
                KeyCode::Enter | KeyCode::Char(' ') => self.activate_control(),
                _ => {}
            },
        }
    }

    fn adjust_control(&mut self, direction: i16, fine: bool) {
        let step = if fine { 1 } else { 5 };
        match self.control_index {
            0 => self.foreground.r = adjust_u8(self.foreground.r, direction, step),
            1 => self.foreground.g = adjust_u8(self.foreground.g, direction, step),
            2 => self.foreground.b = adjust_u8(self.foreground.b, direction, step),
            3 => self.background.r = adjust_u8(self.background.r, direction, step),
            4 => self.background.g = adjust_u8(self.background.g, direction, step),
            5 => self.background.b = adjust_u8(self.background.b, direction, step),
            6 => self.brightness = adjust_level(self.brightness, direction),
            7 => self.speed = adjust_level(self.speed, direction),
            8 => self.direction = cycle_direction(self.direction, direction),
            9 => self.full_color = direction > 0,
            10 => {}
            _ => {}
        }
    }

    fn activate_control(&mut self) {
        match self.control_index {
            9 => self.full_color = !self.full_color,
            _ => self.apply_requested = true,
        }
    }

    fn to_mode(&self) -> Mode {
        match EFFECTS[self.effect_index].1 {
            None => Mode {
                effect: Effect::Static,
                foreground: Rgb::OFF,
                background: Rgb::OFF,
                brightness: 0,
                speed: 0,
                direction: Direction::Right,
                full_color: false,
                logo: false,
            },
            Some(effect) => Mode {
                effect,
                foreground: self.foreground,
                background: self.background,
                brightness: self.brightness,
                speed: self.speed,
                direction: self.direction,
                full_color: if effect == Effect::Custom {
                    true
                } else {
                    self.full_color
                },
                logo: false,
            },
        }
    }
}

struct KeyCell {
    label: &'static str,
    width: u16,
    position: KeyPosition,
    color: Rgb,
}

struct PerKeyState {
    rows: Vec<Vec<KeyCell>>,
    row: usize,
    col: usize,
    editor: Rgb,
    focus: KeyFocus,
    dirty: bool,
    apply_requested: bool,
}

impl PerKeyState {
    fn new() -> Self {
        let rows = KEY_ROWS
            .iter()
            .map(|row| {
                row.iter()
                    .map(|(name, label, width)| KeyCell {
                        label,
                        width: *width,
                        position: key_position(name)
                            .unwrap_or_else(|| panic!("invalid TUI key: {name}")),
                        color: Rgb::OFF,
                    })
                    .collect()
            })
            .collect();

        Self {
            rows,
            row: 0,
            col: 0,
            editor: Rgb::new(0, 128, 255),
            focus: KeyFocus::Grid,
            dirty: false,
            apply_requested: false,
        }
    }

    fn next_focus(&mut self, back: bool) {
        let order = [
            KeyFocus::Grid,
            KeyFocus::Red,
            KeyFocus::Green,
            KeyFocus::Blue,
            KeyFocus::Apply,
        ];
        let index = order
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next = if back {
            (index + order.len() - 1) % order.len()
        } else {
            (index + 1) % order.len()
        };
        self.focus = order[next];
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let fine = key.modifiers.contains(KeyModifiers::SHIFT);
        match self.focus {
            KeyFocus::Grid => match key.code {
                KeyCode::Up => self.move_selection(-1, 0),
                KeyCode::Down => self.move_selection(1, 0),
                KeyCode::Left => self.move_selection(0, -1),
                KeyCode::Right => self.move_selection(0, 1),
                KeyCode::Enter | KeyCode::Char(' ') => self.assign_selected(),
                _ => {}
            },
            KeyFocus::Red | KeyFocus::Green | KeyFocus::Blue => match key.code {
                KeyCode::Left | KeyCode::Down => self.adjust_editor(-1, fine),
                KeyCode::Right | KeyCode::Up => self.adjust_editor(1, fine),
                KeyCode::Enter | KeyCode::Char(' ') => self.assign_selected(),
                _ => {}
            },
            KeyFocus::Apply => match key.code {
                KeyCode::Enter | KeyCode::Char(' ') => self.apply_requested = true,
                _ => {}
            },
        }

        match key.code {
            KeyCode::Char('c') => self.clear_selected(),
            KeyCode::Char('f') => self.fill_all(),
            KeyCode::Char('r') => self.reset(),
            _ => {}
        }
    }

    fn move_selection(&mut self, row_delta: isize, col_delta: isize) {
        if row_delta != 0 {
            let next = (self.row as isize + row_delta)
                .clamp(0, self.rows.len().saturating_sub(1) as isize)
                as usize;
            self.row = next;
            self.col = self.col.min(self.rows[self.row].len().saturating_sub(1));
        }
        if col_delta != 0 {
            let len = self.rows[self.row].len();
            self.col =
                (self.col as isize + col_delta).clamp(0, len.saturating_sub(1) as isize) as usize;
        }
    }

    fn adjust_editor(&mut self, direction: i16, fine: bool) {
        let step = if fine { 1 } else { 5 };
        match self.focus {
            KeyFocus::Red => self.editor.r = adjust_u8(self.editor.r, direction, step),
            KeyFocus::Green => self.editor.g = adjust_u8(self.editor.g, direction, step),
            KeyFocus::Blue => self.editor.b = adjust_u8(self.editor.b, direction, step),
            _ => {}
        }
    }

    fn selected_cell(&self) -> &KeyCell {
        &self.rows[self.row][self.col]
    }

    fn selected_cell_mut(&mut self) -> &mut KeyCell {
        &mut self.rows[self.row][self.col]
    }

    fn assign_selected(&mut self) {
        self.selected_cell_mut().color = self.editor;
        self.dirty = true;
    }

    fn clear_selected(&mut self) {
        self.selected_cell_mut().color = Rgb::OFF;
        self.dirty = true;
    }

    fn fill_all(&mut self) {
        for row in &mut self.rows {
            for cell in row {
                cell.color = self.editor;
            }
        }
        self.dirty = true;
    }

    fn reset(&mut self) {
        for row in &mut self.rows {
            for cell in row {
                cell.color = Rgb::OFF;
            }
        }
        self.dirty = true;
    }

    fn all_keys(&self) -> Vec<(KeyPosition, Rgb)> {
        self.rows
            .iter()
            .flatten()
            .map(|cell| (cell.position, cell.color))
            .collect()
    }
}

struct App {
    device: Device,
    version: String,
    screen: Screen,
    mode: ModeState,
    per_key: PerKeyState,
    status: String,
    status_error: bool,
    quit: bool,
}

impl App {
    fn new(device: Device, version: String) -> Self {
        Self {
            device,
            version,
            screen: Screen::Mode,
            mode: ModeState::new(),
            per_key: PerKeyState::new(),
            status: "Ready".to_string(),
            status_error: false,
            quit: false,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            self.quit = true;
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                self.quit = true;
                return;
            }
            KeyCode::Esc => {
                if self.screen == Screen::Help {
                    self.screen = Screen::Mode;
                } else {
                    self.quit = true;
                }
                return;
            }
            KeyCode::F(1) | KeyCode::Char('m') => {
                self.screen = Screen::Mode;
                return;
            }
            KeyCode::F(2) | KeyCode::Char('k') => {
                self.screen = Screen::PerKey;
                return;
            }
            KeyCode::F(3) | KeyCode::Char('?') => {
                self.screen = Screen::Help;
                return;
            }
            KeyCode::Tab => {
                self.next_focus(false);
                return;
            }
            KeyCode::BackTab => {
                self.next_focus(true);
                return;
            }
            KeyCode::Char('a') => {
                self.apply_current();
                return;
            }
            _ => {}
        }

        match self.screen {
            Screen::Mode => self.mode.handle_key(key),
            Screen::PerKey => self.per_key.handle_key(key),
            Screen::Help => {}
        }

        if self.mode.apply_requested {
            self.mode.apply_requested = false;
            self.apply_mode();
        }
        if self.per_key.apply_requested {
            self.per_key.apply_requested = false;
            self.apply_per_key();
        }
    }

    fn next_focus(&mut self, back: bool) {
        match self.screen {
            Screen::Mode => self.mode.toggle_focus(),
            Screen::PerKey => self.per_key.next_focus(back),
            Screen::Help => {}
        }
    }

    fn apply_current(&mut self) {
        match self.screen {
            Screen::Mode => self.apply_mode(),
            Screen::PerKey => self.apply_per_key(),
            Screen::Help => {}
        }
    }

    fn apply_mode(&mut self) {
        let mode = self.mode.to_mode();
        match self.device.send(&mode.report()) {
            Ok(()) => {
                if mode.brightness == 0 {
                    self.status = format!("Backlight off ({})", self.device.path.display());
                } else {
                    self.status = format!(
                        "Applied {:?} {} brightness {} ({})",
                        mode.effect,
                        mode.foreground,
                        mode.brightness,
                        self.device.path.display()
                    );
                }
                self.status_error = false;
            }
            Err(error) => {
                self.status = format!("Failed to apply: {error}");
                self.status_error = true;
            }
        }
    }

    fn apply_per_key(&mut self) {
        let keys = self.per_key.all_keys();
        let background = Rgb::OFF;
        let mode = Mode {
            effect: Effect::Custom,
            foreground: Rgb::new(255, 255, 255),
            background,
            brightness: self.mode.brightness,
            speed: 0,
            direction: Direction::Right,
            full_color: true,
            logo: false,
        };

        if let Err(error) = self.device.send(&mode.report()) {
            self.status = format!("Failed to enable custom mode: {error}");
            self.status_error = true;
            return;
        }

        for report in custom_reports(&keys, background) {
            if let Err(error) = self.device.send(&report) {
                self.status = format!("Failed to send per-key data: {error}");
                self.status_error = true;
                return;
            }
        }

        self.per_key.dirty = false;
        self.status = format!(
            "Applied per-key colors: {} keys, brightness {} ({})",
            keys.len(),
            self.mode.brightness,
            self.device.path.display()
        );
        self.status_error = false;
    }

    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(area);

        self.draw_header(frame, chunks[0]);
        self.draw_tabs(frame, chunks[1]);
        match self.screen {
            Screen::Mode => self.draw_mode(frame, chunks[2]),
            Screen::PerKey => self.draw_per_key(frame, chunks[2]),
            Screen::Help => self.draw_help(frame, chunks[2]),
        }
        self.draw_footer(frame, chunks[3]);
    }

    fn draw_header(&self, frame: &mut Frame, area: Rect) {
        let line = Line::from(vec![
            Span::styled(
                " ARDOR LED ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                self.device.path.display().to_string(),
                Style::default().fg(Color::Yellow),
            ),
            Span::raw("  "),
            Span::styled(self.device.name.clone(), Style::default().fg(Color::Gray)),
            Span::raw("  "),
            Span::styled(
                format!("firmware: {}", self.version),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(Paragraph::new(line).block(block), area);
    }

    fn draw_tabs(&self, frame: &mut Frame, area: Rect) {
        let tabs = Tabs::new(vec![" Mode ", " Per-key ", " Help "])
            .select(self.screen.index())
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("|");
        frame.render_widget(tabs, area);
    }

    fn draw_mode(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::horizontal([Constraint::Length(22), Constraint::Min(0)]).split(area);

        let items: Vec<ListItem> = EFFECTS
            .iter()
            .map(|(name, _)| ListItem::new(*name))
            .collect();
        let highlight = if self.mode.focus == ModeFocus::Effects {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White).bg(Color::DarkGray)
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Effects ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            )
            .highlight_style(highlight);
        let mut state = ListState::default();
        state.select(Some(self.mode.effect_index));
        frame.render_stateful_widget(list, chunks[0], &mut state);

        let active = self.mode.focus == ModeFocus::Controls;
        let lines = mode_control_lines(&self.mode, active);
        let block = Block::default()
            .title(" Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            chunks[1],
        );
    }

    fn draw_per_key(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::vertical([Constraint::Length(14), Constraint::Min(0)]).split(area);
        self.draw_keyboard(frame, chunks[0]);
        self.draw_key_editor(frame, chunks[1]);
    }

    fn draw_keyboard(&self, frame: &mut Frame, area: Rect) {
        let max_width = self
            .per_key
            .rows
            .iter()
            .map(|row| row.iter().map(|cell| cell.width).sum::<u16>())
            .max()
            .unwrap_or(0);
        let block = Block::default()
            .title(" Keyboard ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));

        if area.width < max_width + 2 {
            let message = format!(
                "Terminal is too narrow for the keyboard layout.\nNeed at least {} columns.",
                max_width + 2
            );
            frame.render_widget(
                Paragraph::new(message)
                    .block(block)
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                area,
            );
            return;
        }

        let mut lines = Vec::new();
        for (row_index, row) in self.per_key.rows.iter().enumerate() {
            let mut spans = Vec::new();
            for (col_index, cell) in row.iter().enumerate() {
                let selected = row_index == self.per_key.row && col_index == self.per_key.col;
                spans.push(Span::styled(
                    format!("{:^width$}", cell.label, width = cell.width as usize),
                    key_style(cell.color, selected),
                ));
            }
            lines.push(Line::from(spans));
            if row_index + 1 < self.per_key.rows.len() {
                lines.push(Line::from(""));
            }
        }

        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .alignment(Alignment::Center),
            area,
        );
    }

    fn draw_key_editor(&self, frame: &mut Frame, area: Rect) {
        let selected = self.per_key.selected_cell();
        let key_focus = self.per_key.focus;
        let active = key_focus != KeyFocus::Grid;
        let dirty = if self.per_key.dirty {
            " *modified*"
        } else {
            ""
        };

        let mut lines = Vec::new();
        lines.push(Line::from(vec![
            Span::styled(" Selected: ", Style::default().fg(Color::Gray)),
            Span::styled(
                selected.label.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!(
                    "{:?}/{}",
                    selected.position.message, selected.position.index
                ),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("  "),
            color_swatch(selected.color),
            Span::raw(" "),
            Span::styled(selected.color.to_string(), Style::default().fg(Color::Gray)),
            Span::styled(dirty, Style::default().fg(Color::Yellow)),
        ]));
        lines.push(Line::from(""));
        lines.push(editor_color_line(
            "R",
            self.per_key.editor.r,
            Color::Rgb(255, 0, 0),
            key_focus == KeyFocus::Red,
        ));
        lines.push(editor_color_line(
            "G",
            self.per_key.editor.g,
            Color::Rgb(0, 255, 0),
            key_focus == KeyFocus::Green,
        ));
        lines.push(editor_color_line(
            "B",
            self.per_key.editor.b,
            Color::Rgb(0, 0, 255),
            key_focus == KeyFocus::Blue,
        ));
        lines.push(Line::from(vec![
            Span::styled(" Editor: ", Style::default().fg(Color::Gray)),
            color_swatch(self.per_key.editor),
            Span::raw(" "),
            Span::styled(
                self.per_key.editor.to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));
        lines.push(Line::from(""));
        let apply_selected = key_focus == KeyFocus::Apply;
        lines.push(Line::from(vec![Span::styled(
            format!(
                " Apply per-key colors (a / Enter), brightness {} ",
                self.mode.brightness
            ),
            if apply_selected && active {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            },
        )]));

        let block = Block::default()
            .title(" Color editor ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn draw_help(&self, frame: &mut Frame, area: Rect) {
        let lines = vec![
            Line::from(Span::styled(
                "Global",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  F1 / m       Mode and effects"),
            Line::from("  F2 / k       Per-key color editor"),
            Line::from("  F3 / ?       This help"),
            Line::from("  Tab          Move focus"),
            Line::from("  q / Esc      Quit (Esc returns from help)"),
            Line::from(""),
            Line::from(Span::styled(
                "Mode screen",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  Up / Down    Select effect or setting"),
            Line::from("  Left / Right Change value (Shift: fine step)"),
            Line::from("  Enter / a    Apply"),
            Line::from(""),
            Line::from(Span::styled(
                "Per-key screen",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  Arrows       Move on the keyboard or change the active slider"),
            Line::from("  Space/Enter  Assign the editor color to the selected key"),
            Line::from("  c            Clear the selected key"),
            Line::from("  f            Fill all keys with the editor color"),
            Line::from("  r            Reset all keys to black"),
            Line::from("  a            Send the per-key colors to the keyboard"),
        ];
        let block = Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray));
        frame.render_widget(
            Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn draw_footer(&self, frame: &mut Frame, area: Rect) {
        let status_style = if self.status_error {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(Color::Green)
        };
        let hints = match self.screen {
            Screen::Mode => {
                "F1 Mode  F2 Per-key  F3 Help  Tab focus  ↑↓ select  ←→ change  a apply  q quit"
            }
            Screen::PerKey => {
                "Arrows move/change  Tab focus  Space assign  c clear  f fill  r reset  a apply"
            }
            Screen::Help => "F1 Mode  F2 Per-key  Esc back  q quit",
        };
        let lines = vec![
            Line::from(Span::styled(format!(" {}", self.status), status_style)),
            Line::from(Span::styled(
                format!(" {}", hints),
                Style::default().fg(Color::DarkGray),
            )),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }
}

fn mode_control_lines(mode: &ModeState, active: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled(" Effect: ", Style::default().fg(Color::Gray)),
        Span::styled(
            EFFECTS[mode.effect_index].0.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    lines.push(color_line(
        "Foreground R",
        mode.foreground.r,
        Color::Rgb(255, 0, 0),
        active,
        mode.control_index == 0,
    ));
    lines.push(color_line(
        "Foreground G",
        mode.foreground.g,
        Color::Rgb(0, 255, 0),
        active,
        mode.control_index == 1,
    ));
    lines.push(color_line(
        "Foreground B",
        mode.foreground.b,
        Color::Rgb(0, 0, 255),
        active,
        mode.control_index == 2,
    ));
    lines.push(Line::from(vec![
        Span::styled(" Foreground: ", Style::default().fg(Color::Gray)),
        color_swatch(mode.foreground),
        Span::raw(" "),
        Span::styled(
            mode.foreground.to_string(),
            Style::default().fg(Color::Gray),
        ),
    ]));
    lines.push(Line::from(""));

    lines.push(color_line(
        "Background R",
        mode.background.r,
        Color::Rgb(255, 0, 0),
        active,
        mode.control_index == 3,
    ));
    lines.push(color_line(
        "Background G",
        mode.background.g,
        Color::Rgb(0, 255, 0),
        active,
        mode.control_index == 4,
    ));
    lines.push(color_line(
        "Background B",
        mode.background.b,
        Color::Rgb(0, 0, 255),
        active,
        mode.control_index == 5,
    ));
    lines.push(Line::from(vec![
        Span::styled(" Background: ", Style::default().fg(Color::Gray)),
        color_swatch(mode.background),
        Span::raw(" "),
        Span::styled(
            mode.background.to_string(),
            Style::default().fg(Color::Gray),
        ),
    ]));
    lines.push(Line::from(""));

    lines.push(level_line(
        "Brightness",
        mode.brightness,
        4,
        active,
        mode.control_index == 6,
    ));
    lines.push(level_line(
        "Speed",
        mode.speed,
        4,
        active,
        mode.control_index == 7,
    ));
    lines.push(Line::from(Span::styled(
        format!(" {:<14} {}", "Direction", direction_name(mode.direction)),
        control_style(active, mode.control_index == 8),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            " {:<14} {}",
            "Full color",
            if mode.full_color { "on" } else { "off" }
        ),
        control_style(active, mode.control_index == 9),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " Apply now (Enter / a) ",
        if active && mode.control_index == 10 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Cyan)
        },
    )));

    lines
}

fn color_line(
    label: &'static str,
    value: u8,
    color: Color,
    active: bool,
    selected: bool,
) -> Line<'static> {
    let width = 24usize;
    let filled = usize::from(value) * width / 255;
    let bar = format!("{}{}", "#".repeat(filled), ".".repeat(width - filled));
    let style = control_style(active, selected);
    let bar_style = if active && selected {
        Style::default().fg(color).bg(Color::DarkGray)
    } else {
        Style::default().fg(color)
    };
    Line::from(vec![
        Span::styled(format!(" {label:<14}"), style),
        Span::styled(bar, bar_style),
        Span::styled(format!(" {value:3}"), style),
    ])
}

fn level_line(
    label: &'static str,
    value: u8,
    max: u8,
    active: bool,
    selected: bool,
) -> Line<'static> {
    let width = usize::from(max);
    let filled = usize::from(value);
    let bar = format!("{}{}", "#".repeat(filled), ".".repeat(width - filled));
    let style = control_style(active, selected);
    let bar_style = if active && selected {
        Style::default().fg(Color::Yellow).bg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Yellow)
    };
    Line::from(vec![
        Span::styled(format!(" {label:<14}"), style),
        Span::styled(bar, bar_style),
        Span::styled(format!(" {value:3}"), style),
    ])
}

fn editor_color_line(
    label: &'static str,
    value: u8,
    color: Color,
    selected: bool,
) -> Line<'static> {
    let width = 28usize;
    let filled = usize::from(value) * width / 255;
    let bar = format!("{}{}", "#".repeat(filled), ".".repeat(width - filled));
    let style = if selected {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    };
    let bar_style = if selected {
        Style::default().fg(color).bg(Color::DarkGray)
    } else {
        Style::default().fg(color)
    };
    Line::from(vec![
        Span::styled(
            format!(" {} {label} ", if selected { ">" } else { " " }),
            style,
        ),
        Span::styled(bar, bar_style),
        Span::styled(format!(" {value:3} "), style),
    ])
}

fn control_style(active: bool, selected: bool) -> Style {
    if active && selected {
        Style::default().bg(Color::DarkGray).fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    }
}

fn color_swatch(rgb: Rgb) -> Span<'static> {
    Span::styled("    ", Style::default().bg(rat_color(rgb)))
}

fn key_style(rgb: Rgb, selected: bool) -> Style {
    let style = Style::default().bg(rat_color(rgb)).fg(if is_light(rgb) {
        Color::Black
    } else {
        Color::White
    });
    if selected {
        style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        style
    }
}

fn rat_color(rgb: Rgb) -> Color {
    Color::Rgb(rgb.r, rgb.g, rgb.b)
}

fn is_light(rgb: Rgb) -> bool {
    (u32::from(rgb.r) * 299 + u32::from(rgb.g) * 587 + u32::from(rgb.b) * 114) / 1000 > 128
}

fn adjust_u8(value: u8, direction: i16, step: i16) -> u8 {
    if direction < 0 {
        value.saturating_sub(u8::try_from(step).unwrap_or(u8::MAX))
    } else {
        value.saturating_add(u8::try_from(step).unwrap_or(u8::MAX))
    }
}

fn adjust_level(value: u8, direction: i16) -> u8 {
    if direction < 0 {
        value.saturating_sub(1).min(4)
    } else {
        value.saturating_add(1).min(4)
    }
}

fn cycle_direction(direction: Direction, delta: i16) -> Direction {
    const DIRECTIONS: [Direction; 8] = [
        Direction::Right,
        Direction::Left,
        Direction::Up,
        Direction::Down,
        Direction::Outside,
        Direction::Inside,
        Direction::Clockwise,
        Direction::CounterClockwise,
    ];
    let index = DIRECTIONS
        .iter()
        .position(|candidate| *candidate == direction)
        .unwrap_or(0);
    let next = if delta < 0 {
        (index + DIRECTIONS.len() - 1) % DIRECTIONS.len()
    } else {
        (index + 1) % DIRECTIONS.len()
    };
    DIRECTIONS[next]
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Right => "right",
        Direction::Left => "left",
        Direction::Up => "up",
        Direction::Down => "down",
        Direction::Outside => "outside",
        Direction::Inside => "inside",
        Direction::Clockwise => "clockwise",
        Direction::CounterClockwise => "counter-clockwise",
    }
}

pub fn run(device: Device) -> Result<(), String> {
    let version = device
        .query_version()
        .unwrap_or_else(|_| "unknown".to_string());

    let mut terminal = ratatui::init();
    let mut app = App::new(device, version);
    let result = run_loop(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, app: &mut App) -> Result<(), String> {
    while !app.quit {
        terminal
            .draw(|frame| app.draw(frame))
            .map_err(|error| format!("failed to draw TUI: {error}"))?;

        if event::poll(Duration::from_millis(100))
            .map_err(|error| format!("failed to poll terminal events: {error}"))?
            && let Event::Key(key) =
                event::read().map_err(|error| format!("failed to read terminal event: {error}"))?
        {
            app.handle_key(key);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_has_87_unique_keys() {
        let mut positions = std::collections::HashSet::new();
        let mut count = 0;
        for row in KEY_ROWS {
            for (name, _, _) in *row {
                let position = key_position(name).unwrap_or_else(|| panic!("missing key: {name}"));
                assert!(
                    positions.insert((position.message, position.index)),
                    "duplicate key: {name}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 87);
    }

    #[test]
    fn per_key_state_starts_dark_and_tracks_changes() {
        let mut state = PerKeyState::new();
        assert_eq!(state.all_keys().len(), 87);
        assert!(state.all_keys().iter().all(|(_, color)| *color == Rgb::OFF));
        state.assign_selected();
        assert_eq!(state.selected_cell().color, state.editor);
        state.clear_selected();
        assert_eq!(state.selected_cell().color, Rgb::OFF);
        state.fill_all();
        assert!(
            state
                .all_keys()
                .iter()
                .all(|(_, color)| *color == state.editor)
        );
        state.reset();
        assert!(state.all_keys().iter().all(|(_, color)| *color == Rgb::OFF));
    }

    #[test]
    fn mode_defaults_to_blue_static() {
        let mode = ModeState::new().to_mode();
        assert_eq!(mode.effect, Effect::Static);
        assert_eq!(mode.foreground, Rgb::new(0, 0, 255));
        assert_eq!(mode.brightness, 4);
    }
}
