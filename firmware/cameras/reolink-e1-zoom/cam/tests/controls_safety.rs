//! Safety at existing public control boundaries; all peripherals are isolated host fixtures.
use cam::ptz::{Axis, Bus, Error, Position, Ptz, fake::FakeBus};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

struct FaultBus {
    bus: FakeBus,
    fail: Arc<AtomicBool>,
}
impl Bus for FaultBus {
    fn read(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Error> {
        if self.fail.swap(false, Ordering::Relaxed) {
            return Err(Error::Bus("transient read fault".into()));
        }
        self.bus.read(reg, buf)
    }
    fn write(&mut self, reg: u8, data: &[u8]) -> Result<(), Error> {
        self.bus.write(reg, data)
    }
    fn sleep(&mut self, d: Duration) {
        self.bus.sleep(d);
    }
}
#[test]
fn failed_absolute_move_invalidates_position_and_stops_both_axes() {
    let fail = Arc::new(AtomicBool::new(false));
    let mut p = Ptz::new(FaultBus {
        bus: FakeBus::new(0, 0),
        fail: fail.clone(),
    });
    p.init().unwrap();
    p.home().unwrap();
    fail.store(true, Ordering::Relaxed);
    assert!(
        p.goto(Position {
            pan: 100,
            tilt: 100
        })
        .is_err()
    );
    assert_eq!(
        p.position(),
        None,
        "partial movement cannot preserve old coordinates"
    );
    assert_eq!(p.bus().bus.regs()[0] & 6, 6, "best-effort stop both motors");
    assert_eq!(p.goto(Position { pan: 0, tilt: 0 }), Err(Error::NotHomed));
    assert!(!p.is_busy(Axis::Pan).unwrap());
}

#[test]
fn autofocus_rejects_an_inverted_driver_window_without_moving() {
    use cam::{
        zoom::zf_table::ZfTable,
        zoom::{Lens, fake::FakeIsp},
    };
    let mut isp = FakeIsp::new(0, 230);
    isp.focus_window = (1000, 900);
    let mut lens = Lens::new(isp);
    let zf = ZfTable::parse(b"<ZFTable>0,e6,400,3fe</ZFTable>").unwrap();
    assert!(lens.autofocus(&zf).is_err());
    assert!(lens.isp().focus_moves().is_empty());
}

struct InterruptBus {
    bus: FakeBus,
    sender: std::os::unix::net::UnixDatagram,
    armed: Arc<AtomicBool>,
}
impl Bus for InterruptBus {
    fn read(&mut self, reg: u8, out: &mut [u8]) -> Result<(), Error> {
        self.bus.read(reg, out)
    }
    fn write(&mut self, reg: u8, data: &[u8]) -> Result<(), Error> {
        self.bus.write(reg, data)
    }
    fn sleep(&mut self, d: Duration) {
        self.bus.sleep(d);
        if d == Duration::from_millis(10) && self.armed.swap(false, Ordering::AcqRel) {
            self.sender.send(b"S").unwrap();
        }
    }
}
#[test]
fn owner_stop_abandons_whole_home_or_move_and_acknowledges_both_axes() {
    use std::os::unix::net::UnixDatagram;
    for home in [true, false] {
        let root = std::env::temp_dir().join(format!("cam-stop-{}-{home}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let endpoint = root.join("owner");
        let owner = cam::control_stop::Owner::bind(&endpoint).unwrap();
        let epoch = owner.epoch();
        let socket = UnixDatagram::bind(root.join("client")).unwrap();
        socket.connect(&endpoint).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let armed = Arc::new(AtomicBool::new(home));
        let mut ptz = Ptz::new(InterruptBus {
            bus: FakeBus::new(3000, 500),
            sender: socket.try_clone().unwrap(),
            armed: armed.clone(),
        });
        ptz.init().unwrap();
        if !home {
            ptz.home().unwrap();
        }
        armed.store(true, Ordering::Release);
        let mut ptz = ptz.with_stop_owner(owner);
        let result = if home {
            ptz.home()
        } else {
            ptz.goto(Position {
                pan: 7000,
                tilt: 1000,
            })
        };
        assert_eq!(result, Err(Error::Cancelled));
        assert_eq!(ptz.position(), None);
        assert_eq!(ptz.bus().bus.regs()[0] & 6, 6);
        let mut reply = [0];
        assert_eq!(socket.recv(&mut reply).unwrap(), 1);
        assert_eq!(reply, *b"S");
        assert_eq!(epoch.load(Ordering::Acquire), 1);
        if home {
            assert_eq!(ptz.bus().bus.moves().len(), 2, "no next chunk after stop");
        }
        assert_eq!(ptz.goto(Position { pan: 1, tilt: 1 }), Err(Error::NotHomed));
        drop(ptz);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn factory_calibration_rejects_positions_outside_motor_travel() {
    use cam::zoom::zf_table::ZfTable;
    for text in [
        "<ZFTable>0,e6,400,ffff</ZFTable>",
        "<ZFTable>0,e6,ffff,3fe</ZFTable>",
    ] {
        assert!(ZfTable::parse(text.as_bytes()).is_err());
    }
}

#[test]
fn driver_rows_cannot_expand_mechanical_zoom_travel() {
    use cam::zoom::{Lens, fake::FakeIsp};
    let mut isp = FakeIsp::new(0, 230);
    isp.rows = vec![(0, 86, 330), (65534, 958, 1086)];
    let mut lens = Lens::new(isp);
    assert!(lens.zoom_to(2000).is_err());
    assert!(lens.isp().sets().is_empty());
}

#[test]
fn calibration_loader_limits_bytes_before_parsing_or_allocating_unbounded_input() {
    let root = std::env::temp_dir().join(format!("cam-cal-limit-{}", std::process::id()));
    std::fs::write(&root, b"12345").unwrap();
    assert!(cam::files::read_device_or_file(&root, 4).is_err());
    assert_eq!(cam::files::read_device_or_file(&root, 5).unwrap(), b"12345");
    std::fs::remove_file(root).unwrap();
}

#[test]
fn long_running_auto_ir_keeps_a_bounded_switch_history() {
    use cam::ir::{self, Auto, Ir, Thresholds, fake::FakeHw};
    let readings: Vec<_> = (0..300)
        .map(|i| if i % 2 == 0 { 1000 } else { 3000 })
        .collect();
    let mut ir = Ir::new(FakeHw::new(0).with_readings(&readings));
    let mut n = 0;
    let switches = ir::run_auto(
        &mut ir,
        Auto::new(
            Thresholds {
                to_day: 1900,
                to_night: 2300,
            },
            Duration::ZERO,
        )
        .unwrap(),
        Duration::from_millis(500),
        &mut || {
            n += 1;
            n <= readings.len()
        },
    )
    .unwrap();
    assert!(switches.len() <= 128);
}
