use std::fmt;

pub const REPORT_LEN: usize = 64;

pub const CMD_SET_MODE: u8 = 0x07;
pub const CMD_SET_LOGO_MODE: u8 = 0x08;
pub const CMD_SET_LED_DATA: u8 = 0x09;
pub const CMD_GET_VERSION: u8 = 0x0d;

const LED_DATA_BYTES: u8 = 0x36;
const LED_DATA_BYTES_LAST: u8 = 0x12;
const LED_MAX_VALUE: u16 = 0xc1;

const MODE_BYTE_KEYS: u8 = 0x0e;
const MODE_BYTE_LOGO: u8 = 0x0d;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const OFF: Self = Self { r: 0, g: 0, b: 0 };

    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim();

        if let Some(rest) = value.strip_prefix('#') {
            return Self::parse_hex(rest);
        }

        if value.contains(',') {
            let parts: Vec<&str> = value.split(',').collect();
            if parts.len() != 3 {
                return Err(format!("invalid RGB triple: {value}"));
            }
            let mut out = [0u8; 3];
            for (i, part) in parts.iter().enumerate() {
                out[i] = part
                    .trim()
                    .parse::<u8>()
                    .map_err(|_| format!("invalid RGB component: {part}"))?;
            }
            return Ok(Self::new(out[0], out[1], out[2]));
        }

        let named = match value.to_ascii_lowercase().as_str() {
            "black" => Some(Self::new(0, 0, 0)),
            "white" => Some(Self::new(255, 255, 255)),
            "red" => Some(Self::new(255, 0, 0)),
            "green" => Some(Self::new(0, 255, 0)),
            "blue" => Some(Self::new(0, 0, 255)),
            "yellow" => Some(Self::new(255, 255, 0)),
            "cyan" | "aqua" => Some(Self::new(0, 255, 255)),
            "magenta" | "pink" => Some(Self::new(255, 0, 255)),
            "orange" => Some(Self::new(255, 128, 0)),
            "purple" => Some(Self::new(128, 0, 255)),
            _ => None,
        };
        if let Some(rgb) = named {
            return Ok(rgb);
        }

        Self::parse_hex(value)
    }

    fn parse_hex(value: &str) -> Result<Self, String> {
        let value = value.strip_prefix("0x").unwrap_or(value);
        if value.len() != 6 {
            return Err(format!("expected RRGGBB color, got: {value}"));
        }
        let r = u8::from_str_radix(&value[0..2], 16)
            .map_err(|_| format!("invalid red component: {value}"))?;
        let g = u8::from_str_radix(&value[2..4], 16)
            .map_err(|_| format!("invalid green component: {value}"))?;
        let b = u8::from_str_radix(&value[4..6], 16)
            .map_err(|_| format!("invalid blue component: {value}"))?;
        Ok(Self::new(r, g, b))
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Static,
    Breathe,
    Wave,
    Neon,
    Snake,
    Reactive,
    Aurora,
    Ripple,
    Custom,
    Music,
}

impl Effect {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "static" | "solid" => Ok(Self::Static),
            "breathe" | "breathing" => Ok(Self::Breathe),
            "wave" => Ok(Self::Wave),
            "neon" => Ok(Self::Neon),
            "snake" => Ok(Self::Snake),
            "reactive" => Ok(Self::Reactive),
            "aurora" => Ok(Self::Aurora),
            "ripple" => Ok(Self::Ripple),
            "custom" | "per-key" | "perkey" => Ok(Self::Custom),
            "music" | "musical" | "rhythm" => Ok(Self::Music),
            _ => Err(format!("unknown effect: {value}")),
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Static => 0,
            Self::Breathe => 1,
            Self::Wave => 2,
            Self::Neon => 3,
            Self::Snake => 5,
            Self::Reactive => 6,
            Self::Aurora => 7,
            Self::Ripple => 8,
            Self::Custom => 10,
            Self::Music => 100,
        }
    }

    pub fn logo_code(self) -> u8 {
        match self {
            Self::Static
            | Self::Custom
            | Self::Music
            | Self::Reactive
            | Self::Aurora
            | Self::Ripple => 0,
            Self::Breathe => 1,
            Self::Wave | Self::Snake => 2,
            Self::Neon => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Right,
    Left,
    Up,
    Down,
    Outside,
    Inside,
    Clockwise,
    CounterClockwise,
}

impl Direction {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "right" | "r" | "0" => Ok(Self::Right),
            "left" | "l" | "1" => Ok(Self::Left),
            "up" | "u" | "2" => Ok(Self::Up),
            "down" | "d" | "3" => Ok(Self::Down),
            "outside" | "out" | "4" => Ok(Self::Outside),
            "inside" | "in" | "5" => Ok(Self::Inside),
            "clockwise" | "cw" | "6" => Ok(Self::Clockwise),
            "counter-clockwise" | "counterclockwise" | "ccw" | "7" => Ok(Self::CounterClockwise),
            _ => Err(format!("unknown direction: {value}")),
        }
    }

    pub fn code_for(self, effect: Effect) -> u8 {
        match effect {
            Effect::Wave => match self {
                Self::Right => 0,
                Self::Left => 1,
                Self::Up => 2,
                Self::Down => 3,
                Self::Outside => 4,
                Self::Inside => 5,
                Self::Clockwise => 6,
                Self::CounterClockwise => 7,
            },
            Effect::Snake => match self {
                Self::Clockwise | Self::Down => 6,
                _ => 7,
            },
            Effect::Aurora | Effect::Ripple => match self {
                Self::Outside | Self::Inside => 4,
                _ => 5,
            },
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mode {
    pub effect: Effect,
    pub foreground: Rgb,
    pub background: Rgb,
    pub brightness: u8,
    pub speed: u8,
    pub direction: Direction,
    pub full_color: bool,
    pub logo: bool,
}

impl Mode {
    pub fn report(&self) -> [u8; REPORT_LEN] {
        let mut buf = [0u8; REPORT_LEN];
        let effect = if self.logo {
            self.effect.logo_code()
        } else {
            self.effect.code()
        };

        buf[0] = 0x01;
        buf[1] = if self.logo {
            CMD_SET_LOGO_MODE
        } else {
            CMD_SET_MODE
        };
        buf[5] = if self.logo {
            MODE_BYTE_LOGO
        } else {
            MODE_BYTE_KEYS
        };
        buf[6] = effect;
        buf[7] = self.brightness.min(4);
        buf[8] = self.speed.min(4);
        buf[9] = self.foreground.r;
        buf[10] = self.foreground.g;
        buf[11] = self.foreground.b;
        buf[12] = self.background.r;
        buf[13] = self.background.g;
        buf[14] = self.background.b;
        buf[15] = self.direction.code_for(self.effect);
        buf[16] = u8::from(self.full_color);
        buf
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyPosition {
    pub message: u8,
    pub index: u8,
}

impl KeyPosition {
    pub const fn new(message: u8, index: u8) -> Self {
        Self { message, index }
    }
}

pub fn key_position(name: &str) -> Option<KeyPosition> {
    let key = name.trim().to_ascii_lowercase();
    let key = key.strip_prefix("key:").unwrap_or(&key);
    let key = key.strip_prefix("key_").unwrap_or(key);
    let key = key.strip_prefix("key-").unwrap_or(key);

    let (message, index) = match key {
        "esc" | "escape" => (0, 0),
        "f1" => (0, 2),
        "f2" => (0, 3),
        "f3" => (0, 4),
        "f4" => (0, 5),
        "f5" => (0, 7),
        "f6" => (0, 8),
        "f7" => (0, 9),
        "f8" => (0, 10),
        "f9" => (0, 11),
        "f10" => (0, 12),
        "f11" => (0, 13),
        "f12" => (0, 14),
        "print" | "printscreen" | "prtsc" | "prtscr" | "print_screen" => (0, 15),
        "scroll" | "scrolllock" | "scrlk" | "scroll_lock" => (0, 16),
        "pause" | "break" => (0, 17),

        "grave" | "backtick" | "tilde" | "`" | "~" => (1, 4),
        "1" => (1, 5),
        "2" => (1, 6),
        "3" => (1, 7),
        "4" => (1, 8),
        "5" => (1, 9),
        "6" => (1, 10),
        "7" => (1, 11),
        "8" => (1, 12),
        "9" => (1, 13),
        "0" => (1, 14),
        "minus" | "-" => (1, 15),
        "equal" | "equals" | "=" | "plus" | "+" => (1, 16),

        "backspace" | "back" => (2, 0),
        "insert" | "ins" => (2, 1),
        "home" => (2, 2),
        "pageup" | "pgup" | "page_up" => (2, 3),
        "tab" => (2, 8),
        "q" => (2, 9),
        "w" => (2, 10),
        "e" => (2, 11),
        "r" => (2, 12),
        "t" => (2, 13),
        "y" => (2, 14),
        "u" => (2, 15),
        "i" => (2, 16),
        "o" => (2, 17),

        "p" => (3, 0),
        "leftbracket" | "lbracket" | "[" | "{" => (3, 1),
        "rightbracket" | "rbracket" | "]" | "}" => (3, 2),
        "backslash" | "\\" | "|" => (3, 3),
        "delete" | "del" => (3, 5),
        "end" => (3, 6),
        "pagedown" | "pgdn" | "page_down" => (3, 7),
        "capslock" | "caps" | "caps_lock" => (3, 12),
        "a" => (3, 14),
        "s" => (3, 15),
        "d" => (3, 16),
        "f" => (3, 17),

        "g" => (4, 0),
        "h" => (4, 1),
        "j" => (4, 2),
        "k" => (4, 3),
        "l" => (4, 4),
        "semicolon" | ";" | ":" => (4, 5),
        "quote" | "apostrophe" | "'" | "\"" => (4, 6),
        "iso_hash" | "hash" | "#" => (4, 7),
        "enter" | "return" => (4, 8),
        "leftshift" | "lshift" | "shift_l" | "left_shift" => (4, 16),
        "iso_backslash" | "iso_back_slash" => (4, 17),

        "z" => (5, 0),
        "x" => (5, 1),
        "c" => (5, 2),
        "v" => (5, 3),
        "b" => (5, 4),
        "n" => (5, 5),
        "m" => (5, 6),
        "comma" | "," | "<" => (5, 7),
        "period" | "dot" | "." | ">" => (5, 8),
        "slash" | "/" | "?" => (5, 9),
        "rightshift" | "rshift" | "shift_r" | "right_shift" => (5, 12),
        "up" | "arrowup" | "arrow_up" => (5, 14),

        "leftctrl" | "lctrl" | "ctrl_l" | "left_ctrl" | "control" => (6, 2),
        "win" | "super" | "meta" | "gui" | "windows" => (6, 3),
        "leftalt" | "lalt" | "alt_l" | "left_alt" => (6, 4),
        "space" | "spacebar" | "space_bar" => (6, 8),
        "rightalt" | "ralt" | "alt_r" | "altgr" | "right_alt" => (6, 12),
        "fn" | "function" => (6, 13),
        "menu" | "apps" | "application" => (6, 14),
        "rightctrl" | "rctrl" | "ctrl_r" | "right_ctrl" => (6, 15),
        "left" | "arrowleft" | "arrow_left" => (6, 17),

        "down" | "arrowdown" | "arrow_down" => (7, 0),
        "right" | "arrowright" | "arrow_right" => (7, 1),
        _ => return None,
    };

    Some(KeyPosition::new(message, index))
}

fn scale_channel(value: u8) -> u8 {
    ((value as u16 * LED_MAX_VALUE) / 0xff) as u8
}

pub fn custom_reports(keys: &[(KeyPosition, Rgb)], background: Rgb) -> Vec<[u8; REPORT_LEN]> {
    let mut reports = Vec::with_capacity(8);

    for message in 0..8u8 {
        let mut report = [0u8; REPORT_LEN];
        report[0] = 0x01;
        report[1] = CMD_SET_LED_DATA;
        report[4] = message;
        report[5] = if message == 7 {
            LED_DATA_BYTES_LAST
        } else {
            LED_DATA_BYTES
        };

        let led_count = if message == 7 { 6 } else { 18 };
        for index in 0..led_count {
            let offset = 6 + usize::from(index) * 3;
            report[offset] = scale_channel(background.r);
            report[offset + 1] = scale_channel(background.g);
            report[offset + 2] = scale_channel(background.b);
        }

        for (position, color) in keys {
            if position.message != message || position.index >= led_count {
                continue;
            }
            let offset = 6 + usize::from(position.index) * 3;
            report[offset] = scale_channel(color.r);
            report[offset + 1] = scale_channel(color.g);
            report[offset + 2] = scale_channel(color.b);
        }

        reports.push(report);
    }

    reports
}

pub fn version_query_report() -> [u8; REPORT_LEN] {
    let mut buf = [0u8; REPORT_LEN];
    buf[0] = 0x01;
    buf[1] = CMD_GET_VERSION;
    buf
}

pub fn parse_version_reply(reply: &[u8]) -> Option<String> {
    if reply.len() < 5 || reply[0] != 0x01 || reply[1] != CMD_GET_VERSION || reply[4] != 0 {
        return None;
    }
    let text = reply[2..]
        .iter()
        .copied()
        .skip_while(|b| *b <= b' ')
        .take_while(|b| *b > 0)
        .map(char::from)
        .collect::<String>();
    if text.is_empty() { None } else { Some(text) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_colors() {
        assert_eq!(Rgb::parse("#ff8000").unwrap(), Rgb::new(255, 128, 0));
        assert_eq!(Rgb::parse("00ff00").unwrap(), Rgb::new(0, 255, 0));
        assert_eq!(Rgb::parse("10, 20, 30").unwrap(), Rgb::new(10, 20, 30));
        assert_eq!(Rgb::parse("blue").unwrap(), Rgb::new(0, 0, 255));
    }

    #[test]
    fn builds_static_report() {
        let mode = Mode {
            effect: Effect::Static,
            foreground: Rgb::new(255, 0, 0),
            background: Rgb::OFF,
            brightness: 4,
            speed: 0,
            direction: Direction::Right,
            full_color: false,
            logo: false,
        };
        let report = mode.report();
        assert_eq!(
            &report[..17],
            &[1, 7, 0, 0, 0, 0x0e, 0, 4, 0, 255, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn parses_version_reply() {
        let mut reply = [0u8; 64];
        reply[0] = 1;
        reply[1] = 0x0d;
        let version = b"2NUC,01,KB,FL,K210ZETRGB,V1.05.02\0";
        reply[5..5 + version.len()].copy_from_slice(version);
        assert_eq!(
            parse_version_reply(&reply).as_deref(),
            Some("2NUC,01,KB,FL,K210ZETRGB,V1.05.02")
        );
    }

    #[test]
    fn maps_tkl_keys() {
        assert_eq!(key_position("esc"), Some(KeyPosition::new(0, 0)));
        assert_eq!(key_position("F12"), Some(KeyPosition::new(0, 14)));
        assert_eq!(key_position("printscreen"), Some(KeyPosition::new(0, 15)));
        assert_eq!(key_position("backtick"), Some(KeyPosition::new(1, 4)));
        assert_eq!(key_position("backspace"), Some(KeyPosition::new(2, 0)));
        assert_eq!(key_position("backslash"), Some(KeyPosition::new(3, 3)));
        assert_eq!(key_position("leftshift"), Some(KeyPosition::new(4, 16)));
        assert_eq!(key_position("space"), Some(KeyPosition::new(6, 8)));
        assert_eq!(key_position("right"), Some(KeyPosition::new(7, 1)));
        assert_eq!(key_position("nope"), None);
    }

    #[test]
    fn all_tkl_keys_are_unique() {
        let keys = [
            "esc",
            "f1",
            "f2",
            "f3",
            "f4",
            "f5",
            "f6",
            "f7",
            "f8",
            "f9",
            "f10",
            "f11",
            "f12",
            "print",
            "scroll",
            "pause",
            "grave",
            "1",
            "2",
            "3",
            "4",
            "5",
            "6",
            "7",
            "8",
            "9",
            "0",
            "minus",
            "equal",
            "backspace",
            "insert",
            "home",
            "pageup",
            "tab",
            "q",
            "w",
            "e",
            "r",
            "t",
            "y",
            "u",
            "i",
            "o",
            "p",
            "leftbracket",
            "rightbracket",
            "backslash",
            "delete",
            "end",
            "pagedown",
            "capslock",
            "a",
            "s",
            "d",
            "f",
            "g",
            "h",
            "j",
            "k",
            "l",
            "semicolon",
            "quote",
            "enter",
            "leftshift",
            "z",
            "x",
            "c",
            "v",
            "b",
            "n",
            "m",
            "comma",
            "period",
            "slash",
            "rightshift",
            "up",
            "leftctrl",
            "win",
            "leftalt",
            "space",
            "rightalt",
            "fn",
            "menu",
            "rightctrl",
            "left",
            "down",
            "right",
        ];
        assert_eq!(keys.len(), 87);

        let mut seen = std::collections::HashSet::new();
        for key in keys {
            let position = key_position(key).unwrap_or_else(|| panic!("missing key: {key}"));
            assert!(position.message < 8, "invalid message for {key}");
            let count = if position.message == 7 { 6 } else { 18 };
            assert!(position.index < count, "invalid index for {key}");
            assert!(
                seen.insert((position.message, position.index)),
                "duplicate position for {key}"
            );
        }
    }

    #[test]
    fn builds_custom_reports() {
        let reports = custom_reports(&[(KeyPosition::new(0, 0), Rgb::new(255, 0, 0))], Rgb::OFF);
        assert_eq!(reports.len(), 8);
        assert_eq!(reports[0][1], CMD_SET_LED_DATA);
        assert_eq!(reports[0][5], LED_DATA_BYTES);
        assert_eq!(reports[0][6], LED_MAX_VALUE as u8);
        assert_eq!(reports[0][7], 0);
        assert_eq!(reports[0][8], 0);
        assert_eq!(reports[7][5], LED_DATA_BYTES_LAST);
        assert!(reports[1][6..].iter().all(|value| *value == 0));
    }
}
