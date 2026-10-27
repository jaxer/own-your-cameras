//! The real MS32006 bus: `/dev/i2c-2` through the `i2c-dev` ioctls, SMBus I2C block transfers, exactly as the
//! vendor `device` does (`_ms32006_i2c_read` @ 0xf94ec, `_ms32006_i2c_write` @ 0xf9608).

use std::os::fd::{AsRawFd, OwnedFd};
use std::time::Duration;

use crate::ptz::{ADDR, Bus, Error};

/// The bus the MS32006 sits on.
pub const DEVICE: &str = "/dev/i2c-2";

const I2C_TENBIT: libc::c_ulong = 0x0704;
const I2C_SLAVE_FORCE: libc::c_ulong = 0x0706;
const I2C_SMBUS: libc::c_ulong = 0x0720;
const SMBUS_READ: u8 = 1;
const SMBUS_WRITE: u8 = 0;
const SMBUS_I2C_BLOCK_DATA: u32 = 8;
/// `I2C_SMBUS_BLOCK_MAX`: payload bytes after the length byte.
const BLOCK_MAX: usize = 32;

/// `struct i2c_smbus_ioctl_data` from `linux/i2c-dev.h`.
#[repr(C)]
struct SmbusIoctl {
    read_write: u8,
    command: u8,
    size: u32,
    data: *mut [u8; BLOCK_MAX + 2],
}

pub struct I2cBus {
    fd: OwnedFd,
}

fn io_err(what: &str, e: std::io::Error) -> Error {
    Error::Bus(format!("{what}: {e}"))
}

impl I2cBus {
    /// Opens `path` (normally `/dev/i2c-2`) and binds it to the MS32006 address.
    pub fn open(path: &str) -> Result<Self, Error> {
        let fd =
            crate::hardware_fd::open_rw(path).map_err(|e| io_err(&format!("open {path}"), e))?;
        // SAFETY: both requests take a plain integer argument.
        let rc = unsafe { libc::ioctl(fd.as_raw_fd(), I2C_TENBIT as _, 0) };
        if rc < 0 {
            return Err(io_err("I2C_TENBIT", std::io::Error::last_os_error()));
        }
        // SAFETY: as above; FORCE because the vendor driver may already claim the address.
        let rc = unsafe {
            libc::ioctl(
                fd.as_raw_fd(),
                I2C_SLAVE_FORCE as _,
                libc::c_ulong::from(ADDR),
            )
        };
        if rc < 0 {
            return Err(io_err("I2C_SLAVE_FORCE", std::io::Error::last_os_error()));
        }
        Ok(Self { fd })
    }

    fn transfer(&mut self, rw: u8, reg: u8, block: &mut [u8; BLOCK_MAX + 2]) -> Result<(), Error> {
        let mut args = SmbusIoctl {
            read_write: rw,
            command: reg,
            size: SMBUS_I2C_BLOCK_DATA,
            data: block,
        };
        // SAFETY: `args` matches the kernel's `i2c_smbus_ioctl_data` and `data` points at a live 34-byte union.
        let rc = unsafe { libc::ioctl(self.fd.as_raw_fd(), I2C_SMBUS as _, &mut args) };
        if rc < 0 {
            return Err(io_err(
                &format!("I2C_SMBUS reg {reg:#04x}"),
                std::io::Error::last_os_error(),
            ));
        }
        Ok(())
    }
}

impl Bus for I2cBus {
    fn read(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Error> {
        if buf.len() > BLOCK_MAX - 1 {
            return Err(Error::Bus(format!(
                "read of {} bytes is too long",
                buf.len()
            )));
        }
        let mut block = [0u8; BLOCK_MAX + 2];
        block[0] = buf.len() as u8;
        self.transfer(SMBUS_READ, reg, &mut block)?;
        if usize::from(block[0]) != buf.len() {
            return Err(Error::Bus(format!(
                "reg {reg:#04x}: asked for {} bytes, got {}",
                buf.len(),
                block[0]
            )));
        }
        buf.copy_from_slice(&block[1..=buf.len()]);
        Ok(())
    }

    fn write(&mut self, reg: u8, data: &[u8]) -> Result<(), Error> {
        if data.len() > BLOCK_MAX - 1 {
            return Err(Error::Bus(format!(
                "write of {} bytes is too long",
                data.len()
            )));
        }
        let mut block = [0u8; BLOCK_MAX + 2];
        block[0] = data.len() as u8;
        block[1..=data.len()].copy_from_slice(data);
        self.transfer(SMBUS_WRITE, reg, &mut block)
    }

    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}
