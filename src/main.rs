use std::env;
use std::path::PathBuf;

mod device;
mod protocol;
mod tui;

use device::Device;
use protocol::{Direction, Effect, Mode, Rgb, custom_reports, key_position};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let device_override = extract_device(&mut args)?;

    if args.is_empty()
        || args
            .iter()
            .any(|arg| matches!(arg.as_str(), "-h" | "--help" | "help"))
    {
        print_help();
        return Ok(());
    }

    if args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-V" | "--version"))
    {
        println!("ardor-led {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let command = args.remove(0);

    match command.as_str() {
        "tui" | "ui" => {
            let device = resolved_device(device_override)?;
            tui::run(device)
        }
        "list" | "devices" | "ls" => list_devices(),
        "version" | "firmware" => {
            let device = resolved_device(device_override)?;
            let version = device
                .query_version()
                .map_err(|error| format!("failed to read firmware version: {error}"))?;
            println!("{version}");
            Ok(())
        }
        "off" => {
            let device = resolved_device(device_override)?;
            let options = parse_mode_options(&args)?;
            let mode = Mode {
                effect: Effect::Static,
                foreground: Rgb::OFF,
                background: Rgb::OFF,
                brightness: 0,
                speed: 0,
                direction: Direction::Right,
                full_color: false,
                logo: options.logo,
            };
            device.send(&mode.report()).map_err(|error| {
                format!(
                    "failed to send report to {}: {error}",
                    device.path.display()
                )
            })?;
            println!("backlight off ({})", device.path.display());
            Ok(())
        }
        "static" | "solid" | "color" => {
            let device = resolved_device(device_override)?;
            let options = parse_mode_options(&args)?;
            apply_static(&device, options, None)
        }
        "effect" => {
            let device = resolved_device(device_override)?;
            let options = parse_mode_options(&args)?;
            apply_effect(&device, options, None)
        }
        "key" => {
            let device = resolved_device(device_override)?;
            let options = parse_mode_options(&args)?;
            apply_single_key(&device, options)
        }
        "keys" | "per-key" | "perkey" => {
            let device = resolved_device(device_override)?;
            let options = parse_mode_options(&args)?;
            apply_keys(&device, options)
        }
        other => {
            if let Ok(effect) = Effect::parse(other) {
                let device = resolved_device(device_override)?;
                let options = parse_mode_options(&args)?;
                return apply_effect(&device, options, Some(effect));
            }
            if let Ok(color) = Rgb::parse(other) {
                let device = resolved_device(device_override)?;
                let options = parse_mode_options(&args)?;
                return apply_static(&device, options, Some(color));
            }
            Err(format!(
                "unknown command: {other}\nrun 'ardor-led --help' for usage"
            ))
        }
    }
}

fn apply_static(
    device: &Device,
    options: ModeOptions,
    forced_color: Option<Rgb>,
) -> Result<(), String> {
    let color = forced_color
        .or(options.color)
        .or_else(|| {
            options
                .positionals
                .first()
                .and_then(|value| Rgb::parse(value).ok())
        })
        .unwrap_or(Rgb::new(0, 0, 255));

    let mode = Mode {
        effect: Effect::Static,
        foreground: color,
        background: options.background.unwrap_or(Rgb::OFF),
        brightness: options.brightness.unwrap_or(4),
        speed: 0,
        direction: Direction::Right,
        full_color: options.full_color,
        logo: options.logo,
    };

    send_mode(device, &mode)?;
    println!(
        "static color {color}, brightness {}{} ({})",
        mode.brightness,
        if mode.logo { ", logo" } else { "" },
        device.path.display()
    );
    Ok(())
}

fn apply_single_key(device: &Device, options: ModeOptions) -> Result<(), String> {
    let name = options.positionals.first().ok_or_else(|| {
        "a key name is required, for example: ardor-led key esc ff0000".to_string()
    })?;
    let position = key_position(name).ok_or_else(|| format!("unknown key: {name}"))?;

    let color = match options.color {
        Some(color) => color,
        None => {
            let value = options
                .positionals
                .get(1)
                .ok_or_else(|| format!("a color is required for key {name}"))?;
            Rgb::parse(value)?
        }
    };

    apply_custom_keys(device, &[(position, color)], options)
}

fn apply_keys(device: &Device, options: ModeOptions) -> Result<(), String> {
    if options.positionals.is_empty() {
        return Err(
            "at least one KEY=COLOR pair is required, for example: ardor-led keys esc=ff0000 a=00ff00"
                .to_string(),
        );
    }

    let mut keys = Vec::new();
    for value in &options.positionals {
        let (name, color) = value
            .split_once('=')
            .or_else(|| value.split_once(':'))
            .ok_or_else(|| format!("expected KEY=COLOR, got: {value}"))?;
        let position = key_position(name).ok_or_else(|| format!("unknown key: {name}"))?;
        let color = Rgb::parse(color)?;
        keys.push((position, color));
    }

    apply_custom_keys(device, &keys, options)
}

fn apply_custom_keys(
    device: &Device,
    keys: &[(protocol::KeyPosition, Rgb)],
    options: ModeOptions,
) -> Result<(), String> {
    let background = options.background.unwrap_or(Rgb::OFF);
    let brightness = options.brightness.unwrap_or(4);
    let mode = Mode {
        effect: Effect::Custom,
        foreground: Rgb::new(255, 255, 255),
        background,
        brightness,
        speed: 0,
        direction: Direction::Right,
        full_color: true,
        logo: false,
    };

    send_mode(device, &mode)?;
    for report in custom_reports(keys, background) {
        device.send(&report).map_err(|error| {
            format!(
                "failed to send per-key data to {}: {error}",
                device.path.display()
            )
        })?;
    }

    println!(
        "per-key colors: {} key(s), brightness {}, background {} ({})",
        keys.len(),
        brightness,
        background,
        device.path.display()
    );
    Ok(())
}

fn apply_effect(
    device: &Device,
    options: ModeOptions,
    forced_effect: Option<Effect>,
) -> Result<(), String> {
    let effect = forced_effect
        .or(options.effect)
        .or_else(|| {
            options
                .positionals
                .first()
                .and_then(|value| Effect::parse(value).ok())
        })
        .ok_or_else(|| "an effect name is required".to_string())?;

    let mode = Mode {
        effect,
        foreground: options.color.unwrap_or(Rgb::new(0, 0, 255)),
        background: options.background.unwrap_or(Rgb::OFF),
        brightness: options.brightness.unwrap_or(4),
        speed: options.speed.unwrap_or(3),
        direction: options.direction.unwrap_or(Direction::Right),
        full_color: options.full_color,
        logo: options.logo,
    };

    send_mode(device, &mode)?;
    println!(
        "effect {effect:?}, color {}, background {}, brightness {}, speed {}{} ({})",
        mode.foreground,
        mode.background,
        mode.brightness,
        mode.speed,
        if mode.logo { ", logo" } else { "" },
        device.path.display()
    );
    Ok(())
}

fn send_mode(device: &Device, mode: &Mode) -> Result<(), String> {
    device.send(&mode.report()).map_err(|error| {
        format!(
            "failed to send report to {}: {error}",
            device.path.display()
        )
    })
}

fn resolved_device(device_override: Option<PathBuf>) -> Result<Device, String> {
    match device_override {
        Some(path) => device::by_path(&path).map_err(|error| error.to_string()),
        None => device::find_control().map_err(|error| error.to_string()),
    }
}

fn list_devices() -> Result<(), String> {
    let devices =
        device::find_all().map_err(|error| format!("failed to enumerate HID devices: {error}"))?;
    if devices.is_empty() {
        println!("No ARDOR GAMING keyboard (0416:b23c) found.");
        return Ok(());
    }

    for device in devices {
        if device.vendor_interface {
            let version = device
                .query_version()
                .unwrap_or_else(|_| "unknown firmware".to_string());
            println!(
                "{} [control] {} ({version})",
                device.path.display(),
                device.name
            );
        } else {
            println!("{} [input] {}", device.path.display(), device.name);
        }
    }
    Ok(())
}

#[derive(Default)]
struct ModeOptions {
    color: Option<Rgb>,
    background: Option<Rgb>,
    brightness: Option<u8>,
    speed: Option<u8>,
    direction: Option<Direction>,
    effect: Option<Effect>,
    full_color: bool,
    logo: bool,
    positionals: Vec<String>,
}

fn parse_mode_options(args: &[String]) -> Result<ModeOptions, String> {
    let mut options = ModeOptions::default();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--color" | "-c" => {
                options.color = Some(Rgb::parse(&value_for(args, &mut index, arg)?)?);
            }
            "--background" | "--bg" | "-g" => {
                options.background = Some(Rgb::parse(&value_for(args, &mut index, arg)?)?);
            }
            "--brightness" | "-b" => {
                options.brightness = Some(level(value_for(args, &mut index, arg)?, arg)?);
            }
            "--speed" | "-s" => {
                options.speed = Some(level(value_for(args, &mut index, arg)?, arg)?);
            }
            "--direction" | "--dir" | "-D" => {
                options.direction = Some(Direction::parse(&value_for(args, &mut index, arg)?)?);
            }
            "--effect" | "-e" => {
                options.effect = Some(Effect::parse(&value_for(args, &mut index, arg)?)?);
            }
            "--full-color" | "--rainbow" => options.full_color = true,
            "--logo" => options.logo = true,
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => options.positionals.push(value.to_string()),
        }
        index += 1;
    }

    Ok(options)
}

fn value_for(args: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn level(value: String, option: &str) -> Result<u8, String> {
    let level = value
        .parse::<u8>()
        .map_err(|_| format!("{option} expects a number from 0 to 4, got: {value}"))?;
    if level > 4 {
        return Err(format!(
            "{option} expects a number from 0 to 4, got: {value}"
        ));
    }
    Ok(level)
}

fn extract_device(args: &mut Vec<String>) -> Result<Option<PathBuf>, String> {
    let mut device = None;
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        if matches!(arg.as_str(), "--device" | "-d") {
            if index + 1 >= args.len() {
                return Err(format!("{arg} requires a path to a hidraw device"));
            }
            device = Some(PathBuf::from(args.remove(index + 1)));
            args.remove(index);
        } else if let Some(value) = arg.strip_prefix("--device=") {
            device = Some(PathBuf::from(value));
            args.remove(index);
        } else {
            index += 1;
        }
    }

    Ok(device)
}

fn print_help() {
    println!(
        "ardor-led {} - backlight control for ARDOR GAMING Immortality (0416:b23c)\n\
\n\
USAGE:\n\
  ardor-led <COMMAND> [OPTIONS]\n\
\n\
COMMANDS:\n\
  tui                   Interactive terminal interface\n\
  static [COLOR]        Set a solid color (default: blue)\n\
  color [COLOR]         Alias for static\n\
  effect NAME           Set a lighting effect\n\
  key KEY COLOR         Set one key color (e.g. key esc ff0000)\n\
  keys KEY=COLOR ...    Set several key colors (e.g. keys esc=ff0000 a=00ff00)\n\
  off                   Turn the backlight off\n\
  version               Read the keyboard firmware version\n\
  list                  List keyboard HID interfaces\n\
\n\
EFFECTS:\n\
  static, breathe, wave, neon, snake, reactive, aurora, ripple, custom, music\n\
\n\
OPTIONS:\n\
  -c, --color COLOR       Foreground color: RRGGBB, #RRGGBB, R,G,B or a name\n\
  -g, --bg COLOR          Background color (effects with two colors)\n\
  -b, --brightness 0-4    Brightness (default: 4)\n\
  -s, --speed 0-4         Animation speed (default: 3)\n\
  -D, --direction NAME    right, left, up, down, outside, inside, clockwise, ccw\n\
      --full-color        Use the full rainbow instead of the selected colors\n\
      --logo              Control the logo LED instead of the keys\n\
  -d, --device PATH       Use a specific /dev/hidrawN instead of auto-detection\n\
  -h, --help              Show this help\n\
\n\
EXAMPLES:\n\
  ardor-led tui\n\
  ardor-led static ff0000\n\
  ardor-led effect wave --color ff0000 --bg 000000 --brightness 3 --speed 2\n\
  ardor-led effect breathe --color 00ffff --brightness 2\n\
  ardor-led key esc ff0000\n\
  ardor-led keys esc=ff0000 a=00ff00 space=0000ff --bg 000000\n\
  ardor-led off\n\
  ardor-led list",
        env!("CARGO_PKG_VERSION")
    );
}
