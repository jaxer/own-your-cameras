//! Qualified T31 channel-2 motor timer. Speed5 only changes the 16-bit period;
//! a count beyond it causes a wrap-length pause. Repair it while motor-owned.

pub fn missed_compare(period: u16, count: u16) -> bool {
    count > period
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
pub struct MotorClock {
    mapping: *mut libc::c_void,
}

#[cfg(all(target_os = "linux", target_arch = "mips"))]
impl MotorClock {
    pub fn open(speed: i32) -> std::io::Result<Self> {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;
        if std::fs::read_to_string("/sys/devices/system/cpu/online")?.trim() != "0"
            || std::fs::read_to_string("/proc/cmdline")?
                .split_whitespace()
                .any(|v| v == "threadirqs")
        {
            return Err(std::io::Error::other(
                "atomic retarget requires qualified single CPU hard IRQs",
            ));
        }
        let memory = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_SYNC | libc::O_CLOEXEC)
            .open("/dev/mem")?;
        // SAFETY: exact qualified T31 MMIO page. Only channel-2 TDFR, TCNT,
        // TCSR below are accessed; watchdog/channel-0 is never touched.
        let mapping = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                4096,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                memory.as_raw_fd(),
                0x10002000,
            )
        };
        if mapping == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }
        let clock = Self { mapping };
        clock.validate(speed)?;
        Ok(clock)
    }
    fn read(&self, offset: usize) -> u16 {
        // SAFETY: retained mapping and fixed aligned channel-2 register offsets.
        self.read_word(offset) as u16
    }
    fn read_word(&self, offset: usize) -> u32 {
        // SAFETY: fixed aligned TCU status register in retained mapping.
        unsafe { std::ptr::read_volatile(self.mapping.cast::<u8>().add(offset).cast::<u32>()) }
    }
    fn write_word(&self, offset: usize, value: u32) {
        // SAFETY: fixed TCU set/clear register; callers supply channel-2 bits
        // only, never a read-modify-write that could alter watchdog state.
        unsafe {
            std::ptr::write_volatile(self.mapping.cast::<u8>().add(offset).cast::<u32>(), value)
        };
        std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
    }
    /// Freeze only pulse issuance, retaining coils, phase, descriptor and speed.
    /// Mask full+half flags before freezing so an already asserted shared TCU
    /// interrupt cannot issue a late step across the absolute-position snapshot.
    pub fn pause(&self) -> std::io::Result<u32> {
        const MASK: u32 = (1 << 2) | (1 << 18);
        let previous = self.read_word(0x30) & MASK;
        self.write_word(0x34, MASK);
        self.write_word(0x18, 1 << 2);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(10);
        while self.read_word(0xf0) & (1 << 2) != 0 {
            if std::time::Instant::now() >= deadline {
                self.resume(previous);
                return Err(std::io::Error::other("motor timer did not freeze"));
            }
            std::hint::spin_loop();
        }
        self.write_word(0x28, MASK);
        if self.read_word(0x20) & (1 << 2) != 0 || self.read_word(0x10) & (1 << 2) != 0 {
            self.resume(previous);
            return Err(std::io::Error::other("motor timer freeze readback differs"));
        }
        Ok(previous)
    }
    /// Move3 reenables the counter while still masked. Resume pulse issuance
    /// after clearing its phase counter; no motor Stop or coil reset is used.
    pub fn resume(&self, previous_mask: u32) {
        const MASK: u32 = (1 << 2) | (1 << 18);
        // SAFETY: channel-2 counter in retained mapping, aligned 16-bit write.
        unsafe { std::ptr::write_volatile(self.mapping.cast::<u8>().add(0x68).cast::<u32>(), 0) };
        self.write_word(0x28, MASK);
        self.write_word(0x38, MASK & !previous_mask);
    }
    fn validate(&self, speed: i32) -> std::io::Result<u16> {
        if !(100..=900).contains(&speed) || self.read(0x6c) != 0x1c {
            return Err(std::io::Error::other(
                "unqualified motor timer configuration",
            ));
        }
        let period = self.read(0x60);
        if period != (375000 / speed) as u16 {
            return Err(std::io::Error::other(
                "motor timer period differs from driver speed",
            ));
        }
        Ok(period)
    }
    pub fn repair(&self, speed: i32) -> std::io::Result<()> {
        let period = self.validate(speed)?;
        if missed_compare(period, self.read(0x68)) {
            // SAFETY: TCNT2 only; clears missed equality without deenergizing
            // coils, changing the driver trajectory, or touching other channels.
            unsafe {
                std::ptr::write_volatile(self.mapping.cast::<u8>().add(0x68).cast::<u32>(), 0)
            };
            std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
        }
        Ok(())
    }
}
#[cfg(all(target_os = "linux", target_arch = "mips"))]
impl Drop for MotorClock {
    fn drop(&mut self) {
        // SAFETY: exactly the owned mapping, once at final drop.
        unsafe {
            libc::munmap(self.mapping, 4096);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repairs_only_a_count_beyond_the_new_period() {
        for period in [416, 1696, 3750] {
            for count in [0, period - 1, period] {
                assert!(!missed_compare(period, count));
            }
            for count in [period + 1, 65535] {
                assert!(missed_compare(period, count));
            }
        }
    }
}
