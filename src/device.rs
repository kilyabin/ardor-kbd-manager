use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

use crate::protocol::{self, REPORT_LEN};

const VENDOR_ID: u32 = 0x0416;
const PRODUCT_ID: u32 = 0xb23c;

const VENDOR_USAGE_PAGE: [u8; 3] = [0x06, 0x1b, 0xff];

#[derive(Clone, Debug)]
pub struct Device {
    pub path: PathBuf,
    pub name: String,
    pub vendor_interface: bool,
}

impl Device {
    pub fn open_for_write(&self) -> io::Result<File> {
        OpenOptions::new().write(true).open(&self.path)
    }

    pub fn send(&self, report: &[u8; REPORT_LEN]) -> io::Result<()> {
        let mut file = self.open_for_write()?;
        file.write_all(report)?;
        file.flush()
    }

    pub fn query_version(&self) -> io::Result<String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(&self.path)?;

        file.write_all(&protocol::version_query_report())?;
        file.flush()?;

        for _ in 0..20 {
            let mut reply = [0u8; REPORT_LEN];
            match file.read(&mut reply) {
                Ok(n) if n >= 5 => {
                    if let Some(version) = protocol::parse_version_reply(&reply[..n]) {
                        return Ok(version);
                    }
                }
                Ok(_) => {}
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                Err(err) => return Err(err),
            }
            sleep(Duration::from_millis(25));
        }

        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "keyboard did not answer the version request",
        ))
    }
}

pub fn find_all() -> io::Result<Vec<Device>> {
    let mut devices = Vec::new();
    let entries = fs::read_dir("/sys/class/hidraw")?;

    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let sysfs = entry.path();
        let uevent = match fs::read_to_string(sysfs.join("device/uevent")) {
            Ok(value) => value,
            Err(_) => continue,
        };

        let (vendor, product) = match parse_hid_id(&uevent) {
            Some(ids) => ids,
            None => continue,
        };
        if vendor != VENDOR_ID || product != PRODUCT_ID {
            continue;
        }

        let hid_name = uevent
            .lines()
            .find_map(|line| line.strip_prefix("HID_NAME="))
            .unwrap_or("Gaming Keyboard")
            .to_string();

        let descriptor = fs::read(sysfs.join("device/report_descriptor")).unwrap_or_default();
        let vendor_interface = descriptor
            .windows(VENDOR_USAGE_PAGE.len())
            .any(|window| window == VENDOR_USAGE_PAGE);

        devices.push(Device {
            path: PathBuf::from(format!("/dev/{name}")),
            name: hid_name,
            vendor_interface,
        });
    }

    devices.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(devices)
}

pub fn find_control() -> io::Result<Device> {
    let devices = find_all()?;
    if devices.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "ARDOR GAMING keyboard (0416:b23c) was not found",
        ));
    }

    if let Some(device) = devices.iter().find(|device| device.vendor_interface) {
        return Ok(device.clone());
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "keyboard was found, but its vendor HID interface is missing",
    ))
}

pub fn by_path(path: &Path) -> io::Result<Device> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("{} does not exist", path.display()),
        ));
    }
    Ok(Device {
        path: path.to_path_buf(),
        name: "user-specified HID device".to_string(),
        vendor_interface: true,
    })
}

fn parse_hid_id(uevent: &str) -> Option<(u32, u32)> {
    let line = uevent.lines().find(|line| line.starts_with("HID_ID="))?;
    let value = line.strip_prefix("HID_ID=")?;
    let mut parts = value.split(':');
    let _bus = parts.next()?;
    let vendor = u32::from_str_radix(parts.next()?, 16).ok()?;
    let product = u32::from_str_radix(parts.next()?, 16).ok()?;
    Some((vendor, product))
}
