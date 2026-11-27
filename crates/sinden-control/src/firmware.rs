//! Sinden's firmware files, checked before they are sent to the driver.
//!
//! A firmware file is Intel HEX for the gun's `ATmega32U4` (32 KiB of
//! flash). The application presents the gun's USB vendor and product,
//! which make its colour (its player for the games): they are in its data
//! as little-endian bytes, vendor then product (`c0 16 01 0f` for
//! `16c0:0f01`, Blue). [`FirmwareImage::usb_ids`] finds them, so that a UI
//! can tell which gun a file is for before writing it.
//!
//! The files are Sinden's (in their official software package): this crate
//! only reads one the user gives. The driver checks the file again.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The gun's flash: 32 KiB.
pub const FLASH_SIZE: usize = 0x8000;

/// Where the bootloader starts: the application is below.
pub const BOOT_SECTION: usize = 0x7000;

/// The largest file read: the driver's request line is 1 MiB, and a
/// whole 32 KiB image in Intel HEX is under 100 KB.
pub const MAX_FILE_SIZE: u64 = 1 << 20;

/// The USB vendors of Sinden's guns.
const VENDORS: [u16; 2] = [0x16c0, 0x16d0];

/// A USB vendor and product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UsbId {
    /// The vendor ID, e.g. `0x16c0`.
    pub vendor: u16,
    /// The product ID, e.g. `0x0f01`.
    pub product: u16,
}

impl UsbId {
    /// The Sinden gun these identifiers are, if any.
    #[must_use]
    pub fn colour(self) -> Option<GunColour> {
        if !VENDORS.contains(&self.vendor) {
            return None;
        }
        Some(match self.product {
            0x0f01 => GunColour::Blue,
            0x0f02 => GunColour::Red,
            0x0f38 => GunColour::Black,
            0x0f39 => GunColour::Player2,
            _ => return None,
        })
    }
}

impl fmt::Display for UsbId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04x}:{:04x}", self.vendor, self.product)
    }
}

/// A Sinden gun's identity, from its USB product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GunColour {
    /// `0f01`.
    Blue,
    /// `0f02`.
    Red,
    /// `0f38`, which the shotgun's player 1 firmware presents too.
    Black,
    /// `0f39`, which the shotgun's player 2 firmware presents too.
    Player2,
}

/// A firmware file parsed: the bytes it gives, by flash address.
#[derive(Clone, PartialEq, Eq)]
pub struct FirmwareImage {
    flash: Vec<Option<u8>>,
}

impl fmt::Debug for FirmwareImage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FirmwareImage")
            .field("bytes", &self.flash.iter().flatten().count())
            .finish_non_exhaustive()
    }
}

/// Why a file is not a firmware image.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HexError {
    /// A line of the file is wrong.
    #[error("line {line}: {kind}")]
    Line {
        /// The line, from 1.
        line: usize,
        /// What is wrong with it.
        kind: LineError,
    },
    /// The file ends without its end-of-file record (truncated).
    #[error("no end-of-file record: the file is truncated")]
    NoEnd,
    /// The file holds no application at address 0.
    #[error("no application at address 0")]
    NoApplication,
}

/// What is wrong with a line of a HEX file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LineError {
    /// It does not start with `:`.
    #[error("not an Intel HEX record")]
    NotARecord,
    /// A character is not a hexadecimal digit, or the digits are odd.
    #[error("not hexadecimal")]
    NotHex,
    /// Its length byte does not match its data.
    #[error("the record's length does not match its data")]
    Length,
    /// Its checksum is wrong.
    #[error("bad checksum")]
    Checksum,
    /// A record type this format does not have.
    #[error("unknown record type {0:02x}")]
    RecordType(u8),
    /// An address record of the wrong shape.
    #[error("bad address record")]
    AddressRecord,
    /// Data past the 32 KiB of flash.
    #[error("data at 0x{0:x}, past the 32 KiB of flash")]
    PastFlash(usize),
    /// The same address given two different values.
    #[error("address 0x{0:x} given two different values")]
    Conflict(usize),
    /// A record after the end-of-file record.
    #[error("a record after the end of the file")]
    AfterEnd,
}

impl FirmwareImage {
    /// Parses an Intel HEX file: every record's checksum and length
    /// checked, the end-of-file record required, the data placed in the
    /// 32 KiB of flash.
    pub fn parse(text: &str) -> Result<Self, HexError> {
        let mut flash = vec![None; FLASH_SIZE];
        let mut base = 0usize;
        let mut ended = false;
        for (index, line) in text.lines().enumerate() {
            let line_error = |kind| HexError::Line {
                line: index + 1,
                kind,
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if ended {
                return Err(line_error(LineError::AfterEnd));
            }
            let record = Record::parse(line).map_err(line_error)?;
            match record.kind {
                0x00 => {
                    let start = base + usize::from(record.address);
                    for (offset, &byte) in record.data.iter().enumerate() {
                        let address = start + offset;
                        let slot = flash
                            .get_mut(address)
                            .ok_or_else(|| line_error(LineError::PastFlash(address)))?;
                        match slot {
                            Some(old) if *old != byte => {
                                return Err(line_error(LineError::Conflict(address)));
                            }
                            _ => *slot = Some(byte),
                        }
                    }
                }
                0x01 => ended = true,
                0x02 | 0x04 => {
                    let [high, low] = record.data[..] else {
                        return Err(line_error(LineError::AddressRecord));
                    };
                    let value = usize::from(u16::from_be_bytes([high, low]));
                    base = if record.kind == 0x02 {
                        value << 4
                    } else {
                        value << 16
                    };
                }
                // Start addresses: meaningless to a bootloader.
                0x03 | 0x05 => {}
                other => return Err(line_error(LineError::RecordType(other))),
            }
        }
        if !ended {
            return Err(HexError::NoEnd);
        }
        if flash[0].is_none() {
            return Err(HexError::NoApplication);
        }
        Ok(Self { flash })
    }

    /// The byte the file gives at `address`, if any.
    #[must_use]
    pub fn byte(&self, address: usize) -> Option<u8> {
        self.flash.get(address).copied().flatten()
    }

    /// The number of bytes of the application (below the boot section)
    /// the file gives.
    #[must_use]
    pub fn application_size(&self) -> usize {
        self.flash[..BOOT_SECTION].iter().flatten().count()
    }

    /// The Sinden USB identifiers the application presents, in the order
    /// found, each once. Empty for a file that is not a Sinden gun's (the
    /// files of Sinden's console adapters).
    #[must_use]
    pub fn usb_ids(&self) -> Vec<UsbId> {
        let mut ids = Vec::new();
        for window in self.flash[..BOOT_SECTION].windows(4) {
            let [Some(v0), Some(v1), Some(p0), Some(p1)] = *window else {
                continue;
            };
            let id = UsbId {
                vendor: u16::from_le_bytes([v0, v1]),
                product: u16::from_le_bytes([p0, p1]),
            };
            if id.colour().is_some() && !ids.contains(&id) {
                ids.push(id);
            }
        }
        ids
    }
}

/// One record: `:LLAAAATT<data>CC`.
struct Record {
    address: u16,
    kind: u8,
    data: Vec<u8>,
}

impl Record {
    fn parse(line: &str) -> Result<Self, LineError> {
        let digits = line.strip_prefix(':').ok_or(LineError::NotARecord)?;
        if digits.len() % 2 != 0 {
            return Err(LineError::NotHex);
        }
        let bytes = (0..digits.len())
            .step_by(2)
            .map(|i| {
                digits
                    .get(i..i + 2)
                    .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                    .ok_or(LineError::NotHex)
            })
            .collect::<Result<Vec<u8>, _>>()?;
        // Length, address (2), type, checksum.
        if bytes.len() < 5 || bytes.len() != usize::from(bytes[0]) + 5 {
            return Err(LineError::Length);
        }
        if bytes.iter().fold(0u8, |sum, &b| sum.wrapping_add(b)) != 0 {
            return Err(LineError::Checksum);
        }
        Ok(Self {
            address: u16::from_be_bytes([bytes[1], bytes[2]]),
            kind: bytes[3],
            data: bytes[4..bytes.len() - 1].to_vec(),
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::fmt::Write as _;

    use super::*;

    /// One record with its checksum.
    pub(crate) fn record(address: u16, kind: u8, data: &[u8]) -> String {
        let [high, low] = address.to_be_bytes();
        let mut bytes = vec![u8::try_from(data.len()).unwrap(), high, low, kind];
        bytes.extend_from_slice(data);
        let checksum = bytes
            .iter()
            .fold(0u8, |sum, &b| sum.wrapping_add(b))
            .wrapping_neg();
        bytes.push(checksum);
        bytes.iter().fold(":".to_owned(), |mut line, b| {
            let _ = write!(line, "{b:02X}");
            line
        }) + "\n"
    }

    /// A small image whose data holds `id`, as a USB device descriptor
    /// does, and a boot section copy.
    pub(crate) fn image_with(id: Option<UsbId>) -> String {
        let mut text = record(
            0x0000,
            0x00,
            &[0x0c, 0x94, 0x5c, 0x00, 0x0c, 0x94, 0x84, 0x00],
        );
        let mut descriptor = vec![0x12, 0x01, 0x00, 0x02, 0xef, 0x02, 0x01, 0x40];
        if let Some(id) = id {
            descriptor.extend(id.vendor.to_le_bytes());
            descriptor.extend(id.product.to_le_bytes());
        }
        text += &record(0x0100, 0x00, &descriptor);
        // The bootloader's copy (Arduino's identifiers, never Sinden's).
        text += &record(0x0000, 0x04, &[0x00, 0x00]);
        text += &record(0x7000, 0x00, &[0x41, 0x23, 0x36, 0x00]);
        text += &record(0x0000, 0x01, &[]);
        text
    }

    const BLUE: UsbId = UsbId {
        vendor: 0x16c0,
        product: 0x0f01,
    };

    #[test]
    fn colours() {
        let id = |vendor, product| UsbId { vendor, product }.colour();
        assert_eq!(id(0x16c0, 0x0f01), Some(GunColour::Blue));
        assert_eq!(id(0x16c0, 0x0f02), Some(GunColour::Red));
        assert_eq!(id(0x16c0, 0x0f38), Some(GunColour::Black));
        assert_eq!(id(0x16d0, 0x0f39), Some(GunColour::Player2));
        assert_eq!(id(0x2341, 0x0036), None, "the bootloader");
        assert_eq!(id(0x16c0, 0x1097), None, "the camera");
        assert_eq!(BLUE.to_string(), "16c0:0f01");
    }

    #[test]
    fn finds_the_identifiers() {
        let image = FirmwareImage::parse(&image_with(Some(BLUE))).unwrap();
        assert_eq!(image.usb_ids(), [BLUE]);
        assert_eq!(image.byte(0x0001), Some(0x94));
        assert_eq!(image.byte(0x7000), Some(0x41));
        assert_eq!(image.byte(0x0009), None);
        assert_eq!(image.application_size(), 8 + 12);
    }

    #[test]
    fn every_colour() {
        for product in [0x0f01, 0x0f02, 0x0f38, 0x0f39] {
            let id = UsbId {
                vendor: 0x16c0,
                product,
            };
            let image = FirmwareImage::parse(&image_with(Some(id))).unwrap();
            assert_eq!(image.usb_ids(), [id]);
        }
    }

    #[test]
    fn a_guncon_file_has_no_identifiers() {
        let image = FirmwareImage::parse(&image_with(None)).unwrap();
        assert_eq!(image.usb_ids(), []);
    }

    #[test]
    fn identifiers_across_records_and_in_crlf_files() {
        // The vendor at the end of one record, the product in the next.
        let mut text = record(0x0000, 0x00, &[0x0c, 0x94, 0xc0, 0x16]);
        text += &record(0x0004, 0x00, &[0x02, 0x0f]);
        text += &record(0x0000, 0x01, &[]);
        let image = FirmwareImage::parse(&text.replace('\n', "\r\n")).unwrap();
        assert_eq!(
            image.usb_ids(),
            [UsbId {
                vendor: 0x16c0,
                product: 0x0f02
            }]
        );
    }

    #[test]
    fn identifiers_in_the_boot_section_do_not_count() {
        let mut text = record(0x0000, 0x00, &[0x0c, 0x94]);
        text += &record(0x7100, 0x00, &[0xc0, 0x16, 0x01, 0x0f]);
        text += &record(0x0000, 0x01, &[]);
        assert_eq!(FirmwareImage::parse(&text).unwrap().usb_ids(), []);
    }

    #[test]
    fn a_gap_breaks_an_identifier() {
        let mut text = record(0x0000, 0x00, &[0x0c, 0x94, 0xc0, 0x16]);
        text += &record(0x0005, 0x00, &[0x01, 0x0f]);
        text += &record(0x0000, 0x01, &[]);
        assert_eq!(FirmwareImage::parse(&text).unwrap().usb_ids(), []);
    }

    fn line_error(text: &str) -> (usize, LineError) {
        match FirmwareImage::parse(text) {
            Err(HexError::Line { line, kind }) => (line, kind),
            other => panic!("expected a line error, got {other:?}"),
        }
    }

    #[test]
    fn bad_checksum() {
        let good = image_with(Some(BLUE));
        let mut lines: Vec<String> = good.lines().map(str::to_owned).collect();
        // Change one data digit of the second record.
        let digit = if &lines[1][10..11] == "0" { "1" } else { "0" };
        lines[1].replace_range(10..11, digit);
        assert_eq!(line_error(&lines.join("\n")), (2, LineError::Checksum));
    }

    #[test]
    fn malformed_lines() {
        let end = record(0, 0x01, &[]);
        assert_eq!(
            line_error(&format!("hello\n{end}")),
            (1, LineError::NotARecord)
        );
        assert_eq!(line_error(&format!(":0G\n{end}")), (1, LineError::NotHex));
        assert_eq!(line_error(&format!(":000\n{end}")), (1, LineError::NotHex));
        assert_eq!(
            line_error(&format!(":00000001\n{end}")),
            (1, LineError::Length)
        );
        // Says 2 bytes, holds 1.
        assert_eq!(line_error(":0200000000FE\n"), (1, LineError::Length));
        assert_eq!(
            line_error(&format!("{}{end}", record(0, 0x06, &[]))),
            (1, LineError::RecordType(6))
        );
        assert_eq!(
            line_error(&format!("{}{end}", record(0, 0x04, &[1]))),
            (1, LineError::AddressRecord)
        );
        assert_eq!(
            line_error(&format!("{end}{}", record(0, 0x00, &[1]))),
            (2, LineError::AfterEnd)
        );
    }

    #[test]
    fn data_past_the_flash_or_conflicting() {
        let end = record(0, 0x01, &[]);
        assert_eq!(
            line_error(&format!("{}{end}", record(0x7ffe, 0x00, &[1, 2, 3]))),
            (1, LineError::PastFlash(0x8000))
        );
        // An extended linear address of 1: 0x10000 and up.
        let far = format!("{}{}{end}", record(0, 0x04, &[0, 1]), record(0, 0x00, &[1]));
        assert_eq!(line_error(&far), (2, LineError::PastFlash(0x10000)));
        // A segment address of 0x0700: 0x7000.
        let segment = format!(
            "{}{}{}{end}",
            record(0, 0x00, &[9]),
            record(0, 0x02, &[0x07, 0x00]),
            record(0, 0x00, &[1])
        );
        assert_eq!(
            FirmwareImage::parse(&segment).unwrap().byte(0x7000),
            Some(1)
        );
        let twice = format!("{}{}{end}", record(0, 0x00, &[1, 2]), record(1, 0x00, &[3]));
        assert_eq!(line_error(&twice), (2, LineError::Conflict(1)));
        let same = format!("{}{}{end}", record(0, 0x00, &[1, 2]), record(1, 0x00, &[2]));
        assert!(FirmwareImage::parse(&same).is_ok(), "the same value again");
    }

    #[test]
    fn truncated_empty_or_without_application() {
        let good = image_with(Some(BLUE));
        let truncated = good.lines().take(2).collect::<Vec<_>>().join("\n");
        assert_eq!(FirmwareImage::parse(&truncated), Err(HexError::NoEnd));
        assert_eq!(FirmwareImage::parse(""), Err(HexError::NoEnd));
        let boot_only = format!("{}{}", record(0x7000, 0x00, &[1]), record(0, 0x01, &[]));
        assert_eq!(
            FirmwareImage::parse(&boot_only),
            Err(HexError::NoApplication)
        );
        // Blank lines and start-address records are accepted.
        let padded = format!("\n{}{good}\n\n", record(0, 0x03, &[0, 0, 0, 0]));
        assert!(FirmwareImage::parse(&padded).is_ok());
    }

    #[test]
    fn error_messages() {
        assert_eq!(
            HexError::Line {
                line: 4,
                kind: LineError::Checksum
            }
            .to_string(),
            "line 4: bad checksum"
        );
        assert_eq!(
            LineError::PastFlash(0x8000).to_string(),
            "data at 0x8000, past the 32 KiB of flash"
        );
    }
}
