//! Hardware capability catalog, confirmed settings and a separate atomic store.
//! Time is injected; values enter persistent state only after adapter acceptance.
use camera_transport::mqtt::{Domain, Entity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const SAVE_DELAY: Duration = Duration::from_secs(10);
pub const SPEED: u16 = 7;
pub const START_SPEED: u16 = 27;
pub const RAMP_UP: u16 = 28;
pub const RAMP_DOWN: u16 = 29;
pub const STOCK_HV: i32 = 3;

pub struct Descriptor {
    pub slot: u16,
    pub key: &'static str,
    pub name: &'static str,
    pub hardware: Option<u32>,
    pub minimum: i32,
    pub maximum: i32,
    pub options: &'static [&'static str],
    pub unit: Option<&'static str>,
    pub audio: bool,
}
macro_rules! control {
    ($slot:expr,$key:expr,$name:expr,$hw:expr,$min:expr,$max:expr,$unit:expr,$audio:expr) => {
        Descriptor {
            slot: $slot,
            key: $key,
            name: $name,
            hardware: $hw,
            minimum: $min,
            maximum: $max,
            options: &[],
            unit: $unit,
            audio: $audio,
        }
    };
}
pub const CATALOG: &[Descriptor] = &[
    control!(0, "mirror", "Mirror", Some(0), 0, 1, None, false),
    control!(1, "flip", "Flip", Some(0), 0, 1, None, false),
    control!(2, "brightness", "Brightness", Some(1), 0, 255, None, false),
    control!(3, "contrast", "Contrast", Some(2), 0, 255, None, false),
    control!(4, "saturation", "Saturation", Some(3), 0, 255, None, false),
    control!(5, "sharpness", "Sharpness", Some(4), 0, 255, None, false),
    Descriptor {
        slot: 6,
        key: "anti_flicker",
        name: "Anti-flicker",
        hardware: Some(5),
        minimum: 0,
        maximum: 2,
        options: &["off", "50hz", "60hz"],
        unit: None,
        audio: false,
    },
    control!(
        SPEED,
        "ptz_speed",
        "PTZ speed",
        None,
        100,
        900,
        Some("steps/s"),
        false
    ),
    control!(
        8,
        "mic_volume",
        "Microphone level",
        Some(10),
        -30,
        120,
        None,
        true
    ),
    control!(
        9,
        "speaker_volume",
        "Speaker level",
        Some(11),
        -30,
        120,
        None,
        true
    ),
    control!(
        10,
        "main_fps_target",
        "Main target FPS",
        Some(20),
        1,
        20,
        Some("fps"),
        false
    ),
    control!(
        11,
        "sub_fps_target",
        "Sub target FPS",
        Some(21),
        1,
        20,
        Some("fps"),
        false
    ),
    control!(
        12,
        "main_bitrate_target",
        "Main target bitrate",
        Some(22),
        32,
        800,
        Some("kbit/s"),
        false
    ),
    control!(
        13,
        "sub_bitrate_target",
        "Sub target bitrate",
        Some(23),
        32,
        240,
        Some("kbit/s"),
        false
    ),
    control!(
        14,
        "main_gop",
        "Main GOP duration",
        Some(24),
        1,
        15,
        Some("s"),
        false
    ),
    control!(
        15,
        "sub_gop",
        "Sub GOP duration",
        Some(25),
        1,
        15,
        Some("s"),
        false
    ),
    control!(
        16,
        "main_idr_multiplier",
        "Main keyframe refresh multiplier",
        Some(26),
        1,
        3,
        None,
        false
    ),
    control!(
        17,
        "sub_idr_multiplier",
        "Sub keyframe refresh multiplier",
        Some(27),
        1,
        3,
        None,
        false
    ),
    control!(18, "hue", "Hue", Some(30), 0, 255, None, false),
    control!(
        19,
        "ae_compensation",
        "Exposure compensation",
        Some(31),
        0,
        255,
        None,
        false
    ),
    control!(
        20,
        "highlight_suppression",
        "Highlight suppression",
        Some(32),
        0,
        10,
        None,
        false
    ),
    control!(
        21,
        "defect_pixel_correction",
        "Defect pixel correction strength",
        Some(33),
        0,
        255,
        None,
        false
    ),
    control!(
        22,
        "dynamic_range_strength",
        "Dynamic range correction strength",
        Some(34),
        0,
        255,
        None,
        false
    ),
    Descriptor {
        slot: 23,
        key: "white_balance",
        name: "White balance",
        hardware: Some(35),
        minimum: 0,
        maximum: 7,
        options: &[
            "auto",
            "daylight",
            "cloudy",
            "incandescent",
            "fluorescent",
            "twilight",
            "shade",
            "warm_fluorescent",
        ],
        unit: None,
        audio: false,
    },
    control!(
        24,
        "dynamic_range_correction",
        "Dynamic range correction",
        Some(36),
        0,
        1,
        None,
        false
    ),
    control!(
        25,
        "spatial_noise_reduction",
        "Spatial noise reduction",
        Some(37),
        0,
        1,
        None,
        false
    ),
    control!(
        26,
        "temporal_luma_noise_reduction",
        "Temporal luma noise reduction",
        Some(38),
        0,
        1,
        None,
        false
    ),
    control!(
        START_SPEED,
        "ptz_start_speed",
        "PTZ start speed",
        None,
        100,
        900,
        Some("steps/s"),
        false
    ),
    control!(
        RAMP_UP,
        "ptz_ramp_up",
        "PTZ acceleration time",
        None,
        0,
        3000,
        Some("ms"),
        false
    ),
    control!(
        RAMP_DOWN,
        "ptz_ramp_down",
        "PTZ deceleration time",
        None,
        0,
        3000,
        Some("ms"),
        false
    ),
];

impl Descriptor {
    fn boolean(&self) -> bool {
        self.slot <= 1 || matches!(self.hardware, Some(36..=38))
    }
    pub fn valid(&self, value: i32) -> bool {
        (self.minimum..=self.maximum).contains(&value)
    }
    pub fn entity(&self) -> Entity {
        Entity {
            id: self.key.into(),
            name: self.name.into(),
            domain: if self.boolean() {
                Domain::Switch
            } else if !self.options.is_empty() {
                Domain::Select
            } else {
                Domain::Number
            },
            minimum: self.minimum,
            maximum: self.maximum,
            options: self.options.iter().map(|s| (*s).into()).collect(),
            unit: self.unit.map(str::to_owned),
            diagnostic: false,
            control_slot: Some(self.slot),
        }
    }
    pub fn text(&self, value: i32) -> String {
        if self.boolean() {
            if value == 0 { "OFF" } else { "ON" }.into()
        } else if self.options.is_empty() {
            value.to_string()
        } else {
            self.options
                .get(value as usize)
                .unwrap_or(&"unknown")
                .to_string()
        }
    }
    pub fn decode(&self, hardware: i32) -> i32 {
        if self.slot <= 1 {
            ((hardware ^ STOCK_HV) >> self.slot) & 1
        } else if self.hardware == Some(35) {
            match hardware {
                0 => 0,
                2..=8 => hardware - 1,
                _ => -1,
            }
        } else {
            hardware
        }
    }
    pub fn encode(&self, value: i32, current: i32) -> i32 {
        if self.slot <= 1 {
            let bit = 1 << self.slot;
            let logical = current ^ STOCK_HV;
            ((logical & !bit) | (value << self.slot)) ^ STOCK_HV
        } else if self.hardware == Some(35) {
            if value == 0 { 0 } else { value + 1 }
        } else {
            value
        }
    }
}
pub fn descriptor(slot: u16) -> Option<&'static Descriptor> {
    CATALOG.iter().find(|d| d.slot == slot)
}

#[derive(Default)]
pub struct Confirmed {
    values: BTreeMap<u16, i32>,
    dirty_since: Option<Duration>,
}
impl Confirmed {
    pub fn get(&self, slot: u16) -> Option<i32> {
        self.values.get(&slot).copied()
    }
    pub fn observe(&mut self, slot: u16, value: i32) -> bool {
        if !descriptor(slot).is_some_and(|d| d.valid(value)) {
            return false;
        }
        self.values.insert(slot, value);
        true
    }
    pub fn applied(&mut self, slot: u16, value: i32, now: Duration) -> bool {
        let changed = self.get(slot) != Some(value);
        if !self.observe(slot, value) {
            return false;
        }
        if changed && self.dirty_since.is_none() {
            self.dirty_since = Some(now);
        }
        true
    }
    pub fn due(&self, now: Duration) -> bool {
        self.dirty_since
            .is_some_and(|at| now.saturating_sub(at) >= SAVE_DELAY)
    }
    pub fn dirty(&self) -> bool {
        self.dirty_since.is_some()
    }
    pub fn saved(&mut self) {
        self.dirty_since = None;
    }
    pub fn records(&self) -> impl Iterator<Item = (&'static Descriptor, i32)> + '_ {
        CATALOG
            .iter()
            .filter_map(|d| self.get(d.slot).map(|v| (d, v)))
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u32,
    values: BTreeMap<String, i32>,
}
pub struct Store {
    path: PathBuf,
    uid: u32,
}
impl Store {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            uid: 0,
        }
    }
    fn parent(&self) -> io::Result<&Path> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("settings parent missing"))?;
        let m = std::fs::symlink_metadata(parent)?;
        if !m.is_dir() || m.uid() != self.uid || m.mode() & 0o7777 != 0o700 {
            return Err(io::Error::other(
                "settings directory must be private and owned by the runtime",
            ));
        }
        Ok(parent)
    }
    pub(crate) fn read_bytes(&self) -> io::Result<Option<Vec<u8>>> {
        self.parent()?;
        let f = match std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let m = f.metadata()?;
        if !m.is_file()
            || m.uid() != self.uid
            || m.mode() & 0o7777 != 0o600
            || m.nlink() != 1
            || m.len() > 16384
        {
            return Err(io::Error::other(
                "settings file must be private, regular and bounded",
            ));
        }
        let mut bytes = Vec::new();
        f.take(16385).read_to_end(&mut bytes)?;
        Ok(Some(bytes))
    }
    pub fn load(&self) -> io::Result<BTreeMap<u16, i32>> {
        let Some(bytes) = self.read_bytes()? else {
            return Ok(BTreeMap::new());
        };
        let doc: Document = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::other("invalid settings document"))?;
        if doc.version != 1 {
            return Err(io::Error::other("unsupported settings version"));
        }
        Ok(CATALOG
            .iter()
            .filter_map(|d| {
                doc.values
                    .get(d.key)
                    .filter(|v| d.valid(**v))
                    .map(|v| (d.slot, *v))
            })
            .collect())
    }
    pub fn save(&self, confirmed: &Confirmed) -> io::Result<()> {
        let values = confirmed
            .records()
            .map(|(d, v)| (d.key.into(), v))
            .collect();
        let bytes = serde_json::to_vec(&Document { version: 1, values })
            .map_err(|_| io::Error::other("cannot encode settings"))?;
        self.write_bytes(&bytes)
    }
    pub(crate) fn write_bytes(&self, bytes: &[u8]) -> io::Result<()> {
        let parent = self.parent()?;
        if bytes.len() > 16384 {
            return Err(io::Error::other("private document exceeds limit"));
        }
        let temp = parent.join(format!(
            ".{}-{}.tmp",
            self.path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| io::Error::other("invalid private document path"))?,
            std::process::id()
        ));
        let mut created = false;
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
                .open(&temp)?;
            created = true;
            file.write_all(bytes)?;
            file.sync_all()?;
            std::fs::rename(&temp, &self.path)?;
            std::fs::File::open(parent)?.sync_all()
        })();
        // Only a temporary file created by this call is disposable. Existing
        // temp data after create_new failure belongs to an earlier operation.
        if result.is_err() && created {
            let _ = std::fs::remove_file(&temp);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logical_orientation_matches_stock_and_preserves_the_other_axis() {
        for raw in 0..4 {
            for axis in 0..2 {
                let d = descriptor(axis).unwrap();
                for value in 0..2 {
                    let updated = d.encode(value, raw);
                    assert_eq!(d.decode(updated), value);
                    let other = descriptor(1 - axis).unwrap();
                    assert_eq!(other.decode(updated), other.decode(raw));
                }
            }
        }
        assert_eq!(descriptor(0).unwrap().decode(3), 0);
        assert_eq!(descriptor(1).unwrap().decode(3), 0);
    }
    #[test]
    fn first_dirty_deadline_does_not_slide_and_unconfirmed_or_invalid_values_do_not_save() {
        let mut s = Confirmed::default();
        assert!(s.observe(2, 128));
        assert!(!s.dirty());
        assert!(!s.applied(2, 256, Duration::ZERO));
        assert_eq!(s.get(2), Some(128));
        assert!(s.applied(2, 129, Duration::from_secs(1)));
        assert!(s.applied(2, 130, Duration::from_secs(9)));
        assert!(!s.due(Duration::from_secs(10)));
        assert!(s.due(Duration::from_secs(11)));
        s.saved();
        assert!(!s.dirty());
    }
    #[test]
    fn safe_store_roundtrips_actual_confirmed_fields_and_refuses_symlinks() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = std::env::temp_dir().join(format!("mi360-settings-test-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut store = Store::new(root.join("settings.json"));
        store.uid = unsafe { libc::geteuid() };
        let mut state = Confirmed::default();
        state.applied(0, 1, Duration::ZERO);
        state.applied(2, 129, Duration::ZERO);
        store.save(&state).unwrap();
        assert_eq!(store.load().unwrap(), BTreeMap::from([(0, 1), (2, 129)]));
        std::fs::remove_file(&store.path).unwrap();
        symlink("absent", &store.path).unwrap();
        assert!(store.load().is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
}
