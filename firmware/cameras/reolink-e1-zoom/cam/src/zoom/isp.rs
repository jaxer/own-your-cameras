//! The real `vendor_isp` transport: ioctls on `/dev/nvt_isp`, exactly as the vendor `device` issues them
//! (`vendor_isp_init` @ 0x153d44 opens the node `O_RDWR`; `vendor_isp_get_common` @ 0x1584dc uses
//! `_IOWR('c', item, 4)`, `vendor_isp_set_common` @ 0x158df4 `_IOW('c', item, 4)`). The argument is a pointer to
//! the item's payload struct. The IQ items (`vendor_isp_{get,set}_iq` @ 0x156868 / 0x1576c4) are the same ioctls with
//! type `'q'`.

use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Duration;

use crate::zoom::{Error, Isp, WORDS};

pub const DEVICE: &str = "/dev/nvt_isp";

const IOC_WRITE: u32 = 0x4000_0000;
const IOC_READ_WRITE: u32 = 0xc000_0000;
/// The size field the vendor encodes (a pointer), not the payload size.
const IOC_SIZE: u32 = 4 << 16;

/// ioctl type of `vendor_isp_*_common`.
const COMMON: u8 = b'c';
/// ioctl type of `vendor_isp_*_iq`.
const IQ: u8 = b'q';
/// ioctl type of `vendor_isp_*_ae` (`vendor_isp_set_ae` @ 0x154a10: `0x400465nn`).
const AE: u8 = b'e';

fn typed(dir: u32, ty: u8, item: u32) -> u32 {
    dir | IOC_SIZE | (u32::from(ty) << 8) | (item & 0xff)
}

#[cfg(test)]
fn request(dir: u32, item: u32) -> u32 {
    typed(dir, COMMON, item)
}

pub struct NvtIsp {
    fd: OwnedFd,
}

impl NvtIsp {
    pub fn open(path: &str) -> Result<Self, Error> {
        let fd = crate::hardware_fd::open_rw(path)
            .map_err(|e| Error::Ioctl(format!("open {path}: {e}")))?;
        Ok(Self { fd })
    }

    fn ioctl(
        &mut self,
        dir: u32,
        ty: u8,
        item: u32,
        words: &mut [u32; WORDS],
    ) -> Result<(), Error> {
        // The ioctl size encodes a pointer, not the copied bytes, so only item/direction pairs whose payload
        // size is known (docs/hardware/zoom-focus.md, docs/hardware/ir.md) reach the kernel.
        let payload = match (ty, item, dir) {
            (COMMON, 30, IOC_READ_WRITE) => 2052,
            (COMMON, 41 | 42, IOC_WRITE | IOC_READ_WRITE) => 20,
            (COMMON, 43, IOC_WRITE | IOC_READ_WRITE) => 132,
            // nvt_isp.ko item-size table +44*8: 12 bytes; stock c6258.
            (COMMON, 44, IOC_WRITE | IOC_READ_WRITE) => 12,
            // nvt_iq.ko iqt_api_get_item_size (.data + 0x5a24 + item*8 + 4): 8 bytes {isp id, level}.
            (IQ, 10 | 11 | 12 | 13 | 14 | 15 | 20, IOC_WRITE | IOC_READ_WRITE) => 8,
            // nvt_ae.ko aet_api_get_item_size (.data + 0x2de8 + item*8 + 4): 8 bytes {isp id, value}.
            (AE, 12 | 14, IOC_WRITE | IOC_READ_WRITE) => 8,
            _ => {
                return Err(Error::Ioctl(format!(
                    "unsupported ISP item {item} type {ty} direction {dir:#x}"
                )));
            }
        };
        if payload > std::mem::size_of_val(words) {
            return Err(Error::Ioctl("ISP payload exceeds backing buffer".into()));
        }
        let req = typed(dir, ty, item);
        // SAFETY: `words` points at a live buffer of WORDS u32s, large enough for this item and direction.
        let rc = unsafe { libc::ioctl(self.fd.as_raw_fd(), req as _, words.as_mut_ptr()) };
        if rc < 0 {
            return Err(Error::Ioctl(format!(
                "ioctl {req:#010x}: {}",
                std::io::Error::last_os_error()
            )));
        }
        Ok(())
    }
}

impl NvtIsp {
    /// `vendor_isp_set_iq(item, words)`.
    pub fn set_iq(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error> {
        let mut copy = Box::new(*words);
        self.ioctl(IOC_WRITE, IQ, item, &mut copy)
    }

    /// `vendor_isp_get_iq(item, words)`; `words[0]` is the ISP id on the way in.
    pub fn get_iq(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error> {
        self.ioctl(IOC_READ_WRITE, IQ, item, words)
    }
}

impl NvtIsp {
    /// `vendor_isp_set_ae(item, words)`.
    pub fn set_ae(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error> {
        let mut copy = Box::new(*words);
        self.ioctl(IOC_WRITE, AE, item, &mut copy)
    }

    /// `vendor_isp_get_ae(item, words)`; `words[0]` is the ISP id on the way in.
    pub fn get_ae(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error> {
        self.ioctl(IOC_READ_WRITE, AE, item, words)
    }
}

impl Isp for NvtIsp {
    fn get_ae(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error> {
        NvtIsp::get_ae(self, item, words)
    }

    fn set_ae(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error> {
        NvtIsp::set_ae(self, item, words)
    }

    fn get_iq(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error> {
        NvtIsp::get_iq(self, item, words)
    }

    fn set_iq(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error> {
        NvtIsp::set_iq(self, item, words)
    }

    fn get(&mut self, item: u32, words: &mut [u32; WORDS]) -> Result<(), Error> {
        self.ioctl(IOC_READ_WRITE, COMMON, item, words)
    }

    fn set(&mut self, item: u32, words: &[u32; WORDS]) -> Result<(), Error> {
        // The driver only reads a set payload; a copy keeps the pointer from aliasing the caller's borrow.
        let mut copy = Box::new(*words);
        self.ioctl(IOC_WRITE, COMMON, item, &mut copy)
    }

    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_match_the_vendor_binary() {
        // literal `movw/movt r1` pairs in `device`
        assert_eq!(request(IOC_WRITE, 42), 0x4004_632a);
        assert_eq!(request(IOC_READ_WRITE, 41), 0xc004_6329);
        assert_eq!(request(IOC_READ_WRITE, 43), 0xc004_632b);
    }

    #[test]
    fn iq_requests_match_the_vendor_binary() {
        // `vendor_isp_set_iq` @ 0x1576c4, item 15 (night mode): movw 0x710f / movt 0x4004
        assert_eq!(typed(IOC_WRITE, IQ, 15), 0x4004_710f);
        // `vendor_isp_get_iq` @ 0x156868 uses 0xc004_71nn
        assert_eq!(typed(IOC_READ_WRITE, IQ, 15), 0xc004_710f);
        // brightness, `vendor_isp_set_iq` jump target @ 0x158190: movw 0x710e / movt 0x4004
        assert_eq!(typed(IOC_WRITE, IQ, 14), 0x4004_710e);
        // anti-flicker, `vendor_isp_set_ae` @ 0x154c54 / get @ 0x1540c8
        assert_eq!(typed(IOC_WRITE, AE, 12), 0x4004_650c);
        assert_eq!(typed(IOC_READ_WRITE, AE, 12), 0xc004_650c);
    }
}

#[cfg(test)]
mod payload_tests {
    use super::*;
    #[test]
    fn unaudited_items_are_rejected_before_ioctl_including_truncated_aliases() {
        let mut isp = NvtIsp::open("/dev/null").unwrap();
        let mut words = [0; WORDS];
        for item in [0, 31, 45, 256 + 42, u32::MAX] {
            assert!(
                isp.get(item, &mut words)
                    .unwrap_err()
                    .to_string()
                    .contains("unsupported")
            );
            assert!(
                isp.set(item, &words)
                    .unwrap_err()
                    .to_string()
                    .contains("unsupported")
            );
        }
        for item in [0, 16, 256 + 15, u32::MAX] {
            assert!(
                isp.get_iq(item, &mut words)
                    .unwrap_err()
                    .to_string()
                    .contains("unsupported")
            );
            assert!(
                isp.set_iq(item, &words)
                    .unwrap_err()
                    .to_string()
                    .contains("unsupported")
            );
        }
        // VA has a known read payload only; a write must not be inferred from its item number.
        assert!(
            isp.set(30, &words)
                .unwrap_err()
                .to_string()
                .contains("unsupported")
        );
        // Known operations reach the real OS and get ENOTTY on /dev/null.
        for item in [30, 41, 42, 43] {
            assert!(
                isp.get(item, &mut words)
                    .unwrap_err()
                    .to_string()
                    .contains("ioctl 0x")
            );
        }
        assert!(
            isp.get_iq(15, &mut words)
                .unwrap_err()
                .to_string()
                .contains("ioctl 0x")
        );
    }
}

#[cfg(test)]
mod orientation_tests {
    use super::*;
    #[test]
    fn audited_sensor_direction_reaches_the_ioctl_boundary() {
        let mut isp = NvtIsp::open("/dev/null").unwrap();
        let mut words = [0; WORDS];
        assert!(
            isp.get(44, &mut words)
                .unwrap_err()
                .to_string()
                .contains("ioctl 0xc004632c")
        );
        assert!(
            isp.set(44, &words)
                .unwrap_err()
                .to_string()
                .contains("ioctl 0x4004632c")
        );
    }
}
