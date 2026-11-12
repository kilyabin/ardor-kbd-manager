# ardor-kbd-manager

Linux backlight control for **ARDOR GAMING Immortality (K210ZETRGB)** and related
Winbond/NUC ZETRGB keyboards.

The project provides a small CLI (`ardor-led`) and a terminal UI (`ardor-led tui`):
static colors, built-in effects, per-key colors, brightness, speed, direction and
firmware version reporting over the keyboard's vendor HID interface.

> Unofficial project. Not affiliated with ARDOR GAMING. Use at your own risk.

## Supported hardware

Tested on:

| Device | USB ID | Layout | Firmware |
| --- | --- | --- | --- |
| ARDOR GAMING Immortality (K210ZETRGB) | `0416:b23c` | ANSI TKL, 87 keys | `2NUC,01,KB,FL,K210ZETRGB,V1.05.02` |

The protocol is shared by several Winbond/NUC ZETRGB keyboards (the vendor app
lists K180/K181/K191/K212/K215/K660/KC104RZETRGB/NP87/LP87 and others). Auto-detection
is currently limited to `0416:b23c`; for another model, point the tool at its
vendor hidraw node with `--device /dev/hidrawN`. Key mapping profiles for other
layouts (60%, 68-key, 104-key, etc.) are not implemented yet.

## Features

- Solid color and brightness control
- Effects: `static`, `breathe`, `wave`, `neon`, `snake`, `reactive`, `aurora`,
  `ripple`, `custom`, `music`
- Per-key colors for the 87-key TKL layout
- Interactive TUI with an on-screen keyboard and RGB sliders
- Firmware version query and hidraw interface listing
- udev rule for non-root access

`music` is the vendor "Musical Rhythm" mode. The vendor application captures
audio on the host, runs an FFT and streams per-LED data to the keyboard. This
project only sends the mode selector, so `music` will not react to sound unless
host-side audio streaming is added.

## Requirements

- Linux with hidraw
- Rust 1.88 or newer (edition 2024)
- Access to the keyboard's vendor hidraw interface

## Build and install

```bash
git clone https://github.com/kilyabin/ardor-kbd-manager.git
cd ardor-kbd-manager

cargo build --release
sudo install -Dm755 target/release/ardor-led /usr/local/bin/ardor-led

sudo install -Dm644 udev/99-ardor-gaming-keyboard.rules \
  /etc/udev/rules.d/99-ardor-gaming-keyboard.rules
sudo udevadm control --reload-rules
sudo udevadm trigger

# Fallback for sessions without systemd-logind uaccess:
sudo usermod -aG input "$USER"
# then log out and back in
```

`cargo install --path .` also works if you only need the binary.

Verify that the keyboard is detected:

```bash
ardor-led list
```

The control interface is the hidraw node with the vendor usage page `0xFF1B`
(usually `/dev/hidraw6`; the exact number can change after replug or reboot).

## CLI usage

```bash
ardor-led tui

ardor-led static ff0000
ardor-led static 0080ff --brightness 3
ardor-led effect wave --color ff0000 --bg 000000 --brightness 3 --speed 2
ardor-led effect breathe --color 00ffff
ardor-led off

ardor-led key esc ff0000
ardor-led keys esc=ff0000 a=00ff00 space=0000ff --bg 000000
ardor-led keys f1=ff0000 f2=00ff00 f3=0000ff

ardor-led version
ardor-led list
```

Colors accept `RRGGBB`, `#RRGGBB`, `R,G,B`, and names such as `red`, `green`,
`blue`, `white`, `cyan`, `magenta`, `yellow`, `orange`, `purple`.

Common options:

| Option | Description |
| --- | --- |
| `-c`, `--color COLOR` | Foreground color |
| `-g`, `--bg COLOR` | Background color |
| `-b`, `--brightness 0-4` | Brightness |
| `-s`, `--speed 0-4` | Animation speed |
| `-D`, `--direction NAME` | `right`, `left`, `up`, `down`, `outside`, `inside`, `clockwise`, `ccw` |
| `--full-color` | Use the full rainbow instead of the selected colors |
| `--logo` | Target the logo LED instead of the keys |
| `-d`, `--device PATH` | Use a specific `/dev/hidrawN` |

Run `ardor-led --help` for the complete list.

## TUI

```bash
ardor-led tui
```

| Key | Action |
| --- | --- |
| `F1` / `m` | Mode and effects screen |
| `F2` / `k` | Per-key editor |
| `F3` / `?` | Help |
| `Tab` | Move focus |
| `q` / `Esc` | Quit |

**Mode screen:** `Up`/`Down` selects an effect or setting, `Left`/`Right`
changes the value (`Shift` for a fine RGB step), `Enter` or `a` applies.

**Per-key screen:** arrows move across the keyboard or change the focused RGB
slider, `Space`/`Enter` assigns the editor color to the selected key, `c` clears
the key, `f` fills all keys with the editor color, `r` resets all keys to black,
`a` sends the per-key colors to the keyboard. Per-key brightness follows the
Mode screen setting.

The TUI uses the keyboard's current state only as a starting point: colors shown
in the per-key editor are local values, because the device does not expose a
read-back command.

## Troubleshooting

- **Permission denied** — install the udev rule, run `sudo udevadm trigger`, and
  make sure your user is in the `input` group (or use a desktop session with
  `uaccess`). Replug the keyboard after changing rules.
- **Keyboard not found** — check `ardor-led list`. The tool looks for
  `0416:b23c` and the vendor usage page `0xFF1B`.
- **Vendor interface missing** — the keyboard was found, but only its normal
  input interfaces are available. Replug it or check the report descriptor.
- **Firmware query timeout** — try `ardor-led version --device /dev/hidrawN`
  with the control interface.
- **Per-key colors look wrong on another model** — the current mapping is for
  the 87-key TKL layout; other models need their own profile.

## Development

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Project layout:

```
src/device.rs    HID discovery, version query, report I/O
src/protocol.rs  Report layout, effects, directions, TKL key map
src/tui.rs       ratatui/crossterm interface
udev/            Access rule for 0416:b23c
```

## License

MIT. See [LICENSE](LICENSE).
