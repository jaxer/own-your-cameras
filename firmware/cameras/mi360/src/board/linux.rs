//! Linux/MIPS board I/O behind the controller hardware seam.
use super::*;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Instant;

const MOTOR_STOP: u32 = 1;
const MOTOR_MOVE: u32 = 3;
const MOTOR_STATUS: u32 = 4;
const MOTOR_SPEED: u32 = 5;
const SPEAKER_CONFIGURE: u32 = 1;
const SPEAKER_ENABLE: u32 = 2;
const SPEAKER_DISABLE: u32 = 3;
#[repr(C)]
#[derive(Default)]
struct RawMotorStatus {
    x: i32,
    y: i32,
    moving: i32,
    speed: i32,
}
const _: () = assert!(std::mem::size_of::<RawMotorStatus>() == 16);
const _: () = assert!(std::mem::size_of::<[i32; 2]>() == 8);
const _: () = assert!(std::mem::size_of::<[i32; 3]>() == 12);
const _: () = assert!(std::mem::size_of::<libc::c_ulong>() == 4);

pub(super) struct Target {
    _lock: File,
    motor: Option<File>,
    motor_clock: Option<crate::motor_clock::MotorClock>,
    seed_worker: Option<std::thread::JoinHandle<io::Result<[i32; 4]>>>,
    gpio: [Option<File>; 5],
    speaker: Option<File>,
    clock: Instant,
}
fn read_small(path: impl AsRef<Path>, cap: u64) -> io::Result<String> {
    let mut text = String::new();
    File::open(path)?.take(cap + 1).read_to_string(&mut text)?;
    if text.len() as u64 > cap {
        return Err(io::Error::other("board diagnostic exceeds limit"));
    }
    Ok(text)
}
fn gpio_level(text: &str) -> io::Result<bool> {
    match text {
        "0\n" | "0" => Ok(false),
        "1\n" | "1" => Ok(true),
        _ => Err(io::Error::other("invalid GPIO level")),
    }
}
fn read_gpio(pin: u32) -> io::Result<bool> {
    gpio_level(&read_small(format!("/sys/class/gpio/gpio{pin}/value"), 8)?)
}
pub(super) fn motor_diagnostics() -> io::Result<MotorStatus> {
    parse_motor_diagnostics(&read_small("/proc/jz/motor/motor_info", 4096)?)
}
fn numeric_pids() -> io::Result<Vec<(u32, std::path::PathBuf)>> {
    let mut result = Vec::new();
    for entry in std::fs::read_dir("/proc")? {
        let entry = entry?;
        if let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<u32>().ok())
        {
            if pid != std::process::id() {
                result.push((pid, entry.path()));
            }
        }
    }
    Ok(result)
}
fn device_owners(device: &str) -> io::Result<Vec<u32>> {
    let mut result = Vec::new();
    for (pid, directory) in numeric_pids()? {
        let entries = match std::fs::read_dir(directory.join("fd")) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e),
        };
        for fd in entries {
            match std::fs::read_link(fd?.path()) {
                Ok(path) if path == Path::new(device) => {
                    result.push(pid);
                    break;
                }
                Ok(_) => (),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
        }
    }
    Ok(result)
}
pub(super) fn snapshot() -> io::Result<BoardSnapshot> {
    let reset = read_small("/sys/class/gpio/gpio57/direction", 8).and_then(|direction| {
        if direction != "in\n" {
            return Err(io::Error::other("reset GPIO57 is not an existing input"));
        }
        read_gpio(57)
    });
    Ok(BoardSnapshot {
        motor: Observation::from_result(motor_diagnostics()),
        yellow_led_high: Observation::from_result(read_gpio(8)),
        blue_led_high: Observation::from_result(read_gpio(9)),
        ir_illumination_high: Observation::from_result(read_gpio(49)),
        reset_gpio57_high: Observation::from_result(reset),
        watchdog_owner_pids: Observation::from_result(device_owners("/dev/watchdog")),
    })
}
fn device(path: &str) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
}
fn call<T>(file: &File, request: u32, argument: &mut T) -> io::Result<()> {
    // SAFETY: requests and exact fixed-width buffer layouts are recovered from
    // this camera's shipped kernel/module; mutable buffers live through ioctl.
    let result = unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            request as libc::c_ulong,
            argument as *mut T,
        )
    };
    if result != 0 {
        return Err(io::Error::other(format!(
            "ioctl {request}: {}",
            io::Error::last_os_error()
        )));
    }
    Ok(())
}
fn noarg(file: &File, request: u32) -> io::Result<()> {
    // SAFETY: these qualified literal requests ignore their third argument.
    let result = unsafe { libc::ioctl(file.as_raw_fd(), request as libc::c_ulong, 0_usize) };
    if result != 0 {
        return Err(io::Error::other(format!(
            "ioctl {request}: {}",
            io::Error::last_os_error()
        )));
    }
    Ok(())
}
impl Target {
    pub(super) fn open() -> io::Result<Self> {
        // Diagnostic identity check complements the caller's exact kernel/image
        // qualification. No board/sensor autodetection or cross-model GPIO use.
        motor_diagnostics()?;
        let sensor = read_small("/proc/jz/sinfo/info", 128)?;
        if sensor.split(':').next_back().map(str::trim) != Some("sc2336") {
            return Err(unsupported("board sensor is not the qualified SC2336"));
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open("/tmp/mi360-camctl.lock")?;
        // SAFETY: retained file descriptor, qualified target flock ABI.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(busy("another controller owns the retained board lock"));
        }
        Ok(Self {
            _lock: lock,
            motor: None,
            motor_clock: None,
            seed_worker: None,
            gpio: std::array::from_fn(|_| None),
            speaker: None,
            clock: Instant::now(),
        })
    }
    fn claim_motor(&mut self, unhomed: bool) -> io::Result<MotorStatus> {
        self.refuse_vendor(&["miio_devicekit"])?;
        let before = if unhomed {
            parse_motor_diagnostics_mode(&read_small("/proc/jz/motor/motor_info", 4096)?, true)?
        } else {
            motor_diagnostics()?
        };
        if before.owner_open || before.moving {
            return Err(busy("motor is owned or moving; no takeover attempted"));
        }
        if self.motor.is_some() {
            return Err(busy("motor already owned internally"));
        }
        // Open performs no calibration. If the first status ioctl fails, local
        // File drop invokes the recovered kernel release/stop path.
        let file = device("/dev/motor")?;
        let mut raw = RawMotorStatus::default();
        call(&file, MOTOR_STATUS, &mut raw)?;
        if raw.moving != 0 && raw.moving != 1 {
            return Err(io::Error::other("invalid motor moving flag"));
        }
        let status = MotorStatus {
            position: Position { x: raw.x, y: raw.y },
            moving: raw.moving != 0,
            speed: raw.speed,
            owner_open: true,
            max_x: before.max_x,
            max_y: before.max_y,
        };
        if !unhomed {
            validate_motor(status)?;
        }
        let motor_clock = crate::motor_clock::MotorClock::open(status.speed)?;
        self.motor = Some(file);
        self.motor_clock = Some(motor_clock);
        Ok(status)
    }
    fn motor(&self) -> io::Result<&File> {
        self.motor
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "motor is not owned"))
    }
    fn slot(pin: u32) -> io::Result<usize> {
        PINS.iter()
            .position(|p| *p == pin)
            .ok_or_else(|| invalid("unsupported output pin"))
    }
}
impl Hardware for Target {
    fn refuse_vendor(&self, names: &[&str]) -> io::Result<()> {
        for (_, path) in numeric_pids()? {
            match read_small(path.join("comm"), 128) {
                Ok(comm) if names.contains(&comm.trim()) => {
                    return Err(busy("vendor still owns this board function"));
                }
                Ok(_) => (),
                Err(e) if e.kind() == io::ErrorKind::NotFound => (),
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn motor_claim(&mut self) -> io::Result<MotorStatus> {
        self.claim_motor(false)
    }
    fn motor_claim_home(&mut self) -> io::Result<MotorStatus> {
        self.claim_motor(true)
    }
    fn seed_home_start(&mut self) -> io::Result<()> {
        if self.seed_worker.is_some() {
            return Err(busy("homing seed already active"));
        }
        let file = self.motor()?.try_clone()?;
        self.seed_worker = Some(
            std::thread::Builder::new()
                .name("motor-home".into())
                .stack_size(64 * 1024)
                .spawn(move || {
                    let mut seed = [2404_i32, 713, 0, 0];
                    call(&file, 2, &mut seed)?;
                    Ok(seed)
                })?,
        );
        Ok(())
    }
    fn seed_home_pending(&self) -> bool {
        self.seed_worker.is_some()
    }
    fn seed_home_poll(&mut self) -> io::Result<Option<[i32; 4]>> {
        let worker = self
            .seed_worker
            .as_ref()
            .ok_or_else(|| io::Error::other("no seed worker"))?;
        if !worker.is_finished() {
            return Ok(None);
        }
        let worker = self.seed_worker.take().unwrap();
        worker
            .join()
            .map_err(|_| io::Error::other("seed worker panicked"))?
            .map(Some)
    }
    fn motor_status(&mut self) -> io::Result<MotorStatus> {
        let mut raw = RawMotorStatus::default();
        call(self.motor()?, MOTOR_STATUS, &mut raw)?;
        if raw.moving != 0 && raw.moving != 1 {
            return Err(io::Error::other("invalid motor moving flag"));
        }
        Ok(MotorStatus {
            position: Position { x: raw.x, y: raw.y },
            moving: raw.moving != 0,
            speed: raw.speed,
            owner_open: true,
            max_x: PAN_TRAVEL,
            max_y: TILT_TRAVEL,
        })
    }
    fn motor_speed(&mut self, mut speed: i32) -> io::Result<()> {
        call(self.motor()?, MOTOR_SPEED, &mut speed)?;
        self.motor_clock
            .as_ref()
            .ok_or_else(|| io::Error::other("motor clock not owned"))?
            .repair(speed)
    }
    fn motor_move(&mut self, movement: RelativeMove) -> io::Result<()> {
        call(self.motor()?, MOTOR_MOVE, &mut [movement.x, movement.y])?;
        self.motor_clock
            .as_ref()
            .ok_or_else(|| io::Error::other("motor clock not owned"))?
            .repair(movement.speed)
    }
    fn motor_retarget(
        &mut self,
        target: Position,
        axes: [i32; 2],
    ) -> io::Result<(MotorStatus, Position)> {
        let previous = self
            .motor_clock
            .as_ref()
            .ok_or_else(|| io::Error::other("motor clock not owned"))?
            .pause()?;
        let result = retarget_at_snapshot(self, target, axes);
        self.motor_clock
            .as_ref()
            .ok_or_else(|| io::Error::other("motor clock lost"))?
            .resume(previous);
        result
    }
    fn motor_stop(&mut self) -> io::Result<()> {
        noarg(self.motor()?, MOTOR_STOP)
    }
    fn motor_release(&mut self) {
        self.motor.take();
        self.motor_clock.take();
    }
    fn gpio_claim(&mut self, pin: u32) -> io::Result<bool> {
        let index = Self::slot(pin)?;
        if self.gpio[index].is_none() {
            if read_small(format!("/sys/class/gpio/gpio{pin}/direction"), 8)? != "out\n" {
                return Err(busy("GPIO must already be exported as output"));
            }
            self.gpio[index] = Some(device(&format!("/sys/class/gpio/gpio{pin}/value"))?);
        }
        self.gpio_read(pin)
    }
    fn gpio_write(&mut self, pin: u32, high: bool) -> io::Result<()> {
        let file = self.gpio[Self::slot(pin)?]
            .as_mut()
            .ok_or_else(|| busy("GPIO is not owned"))?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(if high { b"1" } else { b"0" })
    }
    fn gpio_read(&mut self, pin: u32) -> io::Result<bool> {
        let index = Self::slot(pin)?;
        if let Some(file) = self.gpio[index].as_mut() {
            file.seek(SeekFrom::Start(0))?;
            let mut text = String::new();
            file.take(8).read_to_string(&mut text)?;
            gpio_level(&text)
        } else {
            read_gpio(pin)
        }
    }
    fn gpio_release(&mut self, pin: u32) {
        if let Ok(i) = Self::slot(pin) {
            self.gpio[i].take();
        }
    }
    fn speaker_claim(&mut self) -> io::Result<()> {
        if !device_owners("/dev/speaker")?.is_empty() {
            return Err(busy("another process holds speaker control"));
        }
        self.speaker = Some(device("/dev/speaker")?);
        call(
            self.speaker.as_ref().unwrap(),
            SPEAKER_CONFIGURE,
            &mut [63_i32, 3, 1],
        )
    }
    fn speaker_gate(&mut self, on: bool) -> io::Result<()> {
        match self.speaker.as_ref() {
            Some(file) => noarg(file, if on { SPEAKER_ENABLE } else { SPEAKER_DISABLE }),
            None if !on => Ok(()),
            None => Err(busy("speaker is not owned")),
        }
    }
    fn speaker_release(&mut self) {
        self.speaker.take();
    }
    fn now_ms(&self) -> u64 {
        self.clock
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
    fn wait_ms(&mut self, milliseconds: u64) {
        std::thread::sleep(Duration::from_millis(milliseconds));
    }
}
