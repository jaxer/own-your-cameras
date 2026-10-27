//! The preset table the hardware thread moves to and saves into: `cam.toml`'s presets with the saved positions
//! of the daemon-owned overlay file applied by name. `cam.toml` is never written; a hand edit of a preset there
//! is newer than an earlier save and wins over it.
use crate::settings::Disk;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const MAX_FILE: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub pan: u32,
    pub tilt: u32,
    pub zoom: u32,
}

/// A preset's name is 1–64 bytes and its position lies within travel (`cam.toml` and the overlay alike).
pub fn check(name: &str, p: &Preset) -> Result<(), String> {
    if name.len() > 64
        || name.trim().is_empty()
        || p.pan > crate::ptz::PAN_MAX
        || p.tilt > crate::ptz::TILT_MAX
        || p.zoom > crate::mqtt::ZOOM_MAX
    {
        return Err("invalid preset name or position".into());
    }
    Ok(())
}

/// One overlay entry: the saved position and the `cam.toml` position it replaced.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Entry {
    pan: u32,
    tilt: u32,
    zoom: u32,
    base: Preset,
}

impl Entry {
    fn at(&self) -> Preset {
        Preset {
            pan: self.pan,
            tilt: self.tilt,
            zoom: self.zoom,
        }
    }
}

pub struct Presets {
    base: BTreeMap<String, Preset>,
    /// Saved positions in force, each for a name of `base`.
    saved: BTreeMap<String, Preset>,
    disk: Box<dyn Disk>,
}

impl Presets {
    /// `cam.toml`'s presets with the overlay on `disk` applied; never fails. An absent overlay is normal; an
    /// unreadable or invalid one is ignored as a whole. Returns the problems to log once.
    pub fn load(base: BTreeMap<String, Preset>, mut disk: Box<dyn Disk>) -> (Self, Vec<String>) {
        let mut problems = Vec::new();
        let entries = match disk.read() {
            Ok(None) => BTreeMap::new(),
            Ok(Some(text)) => parse(&text).unwrap_or_else(|e| {
                problems.push(format!("saved presets ignored: {e}"));
                BTreeMap::new()
            }),
            Err(e) => {
                problems.push(format!("saved presets unreadable: {e}"));
                BTreeMap::new()
            }
        };
        let mut saved = BTreeMap::new();
        for (name, e) in entries {
            match base.get(&name) {
                None => problems.push(format!("saved preset {name} ignored: not in cam.toml")),
                Some(b) if *b != e.base => problems.push(format!(
                    "saved preset {name} ignored: edited in cam.toml since the save"
                )),
                Some(_) => {
                    saved.insert(name, e.at());
                }
            }
        }
        (Self { base, saved, disk }, problems)
    }

    /// The position in force for `name`: the saved one, else `cam.toml`'s.
    pub fn get(&self, name: &str) -> Option<Preset> {
        self.saved
            .get(name)
            .or_else(|| self.base.get(name))
            .copied()
    }

    /// Writes `at` as `name`'s position to the overlay now, then puts it in force. On an error nothing changes.
    pub fn save(&mut self, name: &str, at: Preset) -> Result<(), String> {
        if !self.base.contains_key(name) {
            return Err("unknown preset".into());
        }
        check(name, &at)?;
        let mut next = self.saved.clone();
        next.insert(name.to_owned(), at);
        let text = self.render(&next)?;
        self.write(text.as_bytes()).map_err(|e| e.to_string())?;
        self.saved = next;
        Ok(())
    }

    fn render(&self, saved: &BTreeMap<String, Preset>) -> Result<String, String> {
        let entries: BTreeMap<&str, Entry> = saved
            .iter()
            .filter_map(|(name, p)| {
                let base = *self.base.get(name)?;
                Some((
                    name.as_str(),
                    Entry {
                        pan: p.pan,
                        tilt: p.tilt,
                        zoom: p.zoom,
                        base,
                    },
                ))
            })
            .collect();
        let text = format!(
            "# Written by cam: saved preset positions. `base` is the cam.toml position a save replaced;\n\
             # editing that preset in cam.toml makes the save void.\n{}",
            toml::to_string(&entries).map_err(|e| e.to_string())?
        );
        if text.len() > MAX_FILE {
            return Err(format!("{} bytes, over the {MAX_FILE} limit", text.len()));
        }
        Ok(text)
    }

    fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.disk.write_temp(bytes)?;
        self.disk.sync_temp()?;
        self.disk.rename_temp()?;
        self.disk.sync_dir()
    }
}

fn parse(text: &str) -> Result<BTreeMap<String, Entry>, String> {
    if text.len() > MAX_FILE {
        return Err(format!("{} bytes, over the {MAX_FILE} limit", text.len()));
    }
    // TOML errors quote input lines; keep only the fact.
    let entries: BTreeMap<String, Entry> =
        toml::from_str(text).map_err(|_| "not a valid presets file".to_string())?;
    for (name, e) in &entries {
        check(name, &e.at())?;
        check(name, &e.base)?;
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::FakeDisk;
    use std::sync::{Arc, Mutex};

    /// A fake disk the test keeps a handle on after `Presets` took it.
    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<FakeDisk>>);
    impl Shared {
        fn with(text: &str) -> Self {
            let s = Self::default();
            s.0.lock().unwrap().file = Some(text.into());
            s
        }
        fn file(&self) -> Option<String> {
            self.0.lock().unwrap().file.clone()
        }
    }
    impl Disk for Shared {
        fn read(&mut self) -> std::io::Result<Option<String>> {
            self.0.lock().unwrap().read()
        }
        fn write_temp(&mut self, b: &[u8]) -> std::io::Result<()> {
            self.0.lock().unwrap().write_temp(b)
        }
        fn sync_temp(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().sync_temp()
        }
        fn rename_temp(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().rename_temp()
        }
        fn sync_dir(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().sync_dir()
        }
    }

    fn p(pan: u32, tilt: u32, zoom: u32) -> Preset {
        Preset { pan, tilt, zoom }
    }

    fn base() -> BTreeMap<String, Preset> {
        [
            ("Door".into(), p(6195, 650, 0)),
            ("Desk".into(), p(100, 200, 300)),
        ]
        .into()
    }

    #[test]
    fn without_an_overlay_the_positions_are_cam_toml_s() {
        let disk = Shared::default();
        let (presets, problems) = Presets::load(base(), Box::new(disk.clone()));
        assert!(problems.is_empty());
        assert_eq!(presets.get("Door"), Some(p(6195, 650, 0)));
        assert_eq!(presets.get("nosuch"), None);
        assert_eq!(disk.file(), None);
    }

    #[test]
    fn a_save_is_in_force_at_once_and_after_a_reload() {
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(base(), Box::new(disk.clone()));
        presets.save("Door", p(6100, 700, 256)).unwrap();
        assert_eq!(presets.get("Door"), Some(p(6100, 700, 256)));
        assert_eq!(presets.get("Desk"), Some(p(100, 200, 300)));
        assert_eq!(
            disk.0.lock().unwrap().ops,
            ["read", "write_temp", "sync_temp", "rename_temp", "sync_dir"]
        );
        let text = disk.file().unwrap();
        assert!(text.contains("[Door]"), "{text}");
        let (again, problems) = Presets::load(base(), Box::new(disk));
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(again.get("Door"), Some(p(6100, 700, 256)));
    }

    #[test]
    fn the_overlay_records_the_position_it_replaced() {
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(base(), Box::new(disk.clone()));
        presets.save("Door", p(6100, 700, 256)).unwrap();
        let entries = parse(&disk.file().unwrap()).unwrap();
        assert_eq!(entries["Door"].base, p(6195, 650, 0));
        assert_eq!(entries["Door"].at(), p(6100, 700, 256));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn a_newer_hand_edit_in_cam_toml_wins_and_drops_the_entry_at_the_next_write() {
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(base(), Box::new(disk.clone()));
        presets.save("Door", p(6100, 700, 256)).unwrap();
        let mut edited = base();
        edited.insert("Door".into(), p(5000, 600, 100));
        let (mut presets, problems) = Presets::load(edited, Box::new(disk.clone()));
        assert_eq!(presets.get("Door"), Some(p(5000, 600, 100)));
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(problems[0].contains("Door"));
        presets.save("Desk", p(1, 2, 3)).unwrap();
        let entries = parse(&disk.file().unwrap()).unwrap();
        assert_eq!(entries.keys().collect::<Vec<_>>(), ["Desk"]);
    }

    #[test]
    fn an_entry_for_a_removed_name_is_ignored() {
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(base(), Box::new(disk.clone()));
        presets.save("Door", p(6100, 700, 256)).unwrap();
        let mut fewer = base();
        fewer.remove("Door");
        let (presets, problems) = Presets::load(fewer, Box::new(disk));
        assert_eq!(presets.get("Door"), None);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("not in cam.toml"), "{problems:?}");
    }

    #[test]
    fn an_invalid_overlay_is_ignored_as_a_whole_with_one_problem() {
        for text in [
            "not toml [",
            "[Door]\npan = 1\ntilt = 1\nzoom = 1\nbase = { pan = 6195, tilt = 650, zoom = 0 }\nfocus = 3\n",
            "[Door]\npan = 999999\ntilt = 1\nzoom = 1\nbase = { pan = 6195, tilt = 650, zoom = 0 }\n",
            "[Desk]\npan = 1\ntilt = 1\nzoom = 1\nbase = { pan = 100, tilt = 200, zoom = 300 }\n\
             [Door]\npan = 1\ntilt = 1\nzoom = 99999\nbase = { pan = 6195, tilt = 650, zoom = 0 }\n",
        ] {
            let (presets, problems) = Presets::load(base(), Box::new(Shared::with(text)));
            assert_eq!(problems.len(), 1, "{text}: {problems:?}");
            assert_eq!(presets.get("Door"), Some(p(6195, 650, 0)), "{text}");
            assert_eq!(presets.get("Desk"), Some(p(100, 200, 300)), "{text}");
        }
    }

    #[test]
    fn an_unreadable_overlay_is_one_problem() {
        let disk = Shared::default();
        disk.0.lock().unwrap().fail_read = true;
        let (presets, problems) = Presets::load(base(), Box::new(disk));
        assert_eq!(problems.len(), 1);
        assert_eq!(presets.get("Door"), Some(p(6195, 650, 0)));
    }

    #[test]
    fn an_overlay_over_16_kib_is_ignored() {
        let mut text = String::from(
            "[Door]\npan = 1\ntilt = 1\nzoom = 1\nbase = { pan = 6195, tilt = 650, zoom = 0 }\n",
        );
        text.push_str(&"#".repeat(MAX_FILE));
        let (presets, problems) = Presets::load(base(), Box::new(Shared::with(&text)));
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("limit"), "{problems:?}");
        assert_eq!(presets.get("Door"), Some(p(6195, 650, 0)));
    }

    #[test]
    fn a_failed_write_keeps_the_old_position() {
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(base(), Box::new(disk.clone()));
        presets.save("Door", p(6100, 700, 256)).unwrap();
        disk.0.lock().unwrap().fail_write = true;
        assert!(presets.save("Door", p(1, 2, 3)).is_err());
        assert_eq!(presets.get("Door"), Some(p(6100, 700, 256)));
    }

    #[test]
    fn an_unknown_name_or_an_out_of_travel_position_is_not_saved() {
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(base(), Box::new(disk.clone()));
        assert_eq!(
            presets.save("nosuch", p(1, 2, 3)),
            Err("unknown preset".into())
        );
        assert!(presets.save("Door", p(u32::MAX, 2, 3)).is_err());
        assert_eq!(disk.file(), None);
    }

    #[test]
    fn names_that_need_quoting_round_trip() {
        let mut b = base();
        b.insert("Living room \"sofa\"".into(), p(1, 1, 1));
        let disk = Shared::default();
        let (mut presets, _) = Presets::load(b.clone(), Box::new(disk.clone()));
        presets.save("Living room \"sofa\"", p(2, 2, 2)).unwrap();
        let (again, problems) = Presets::load(b, Box::new(disk));
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(again.get("Living room \"sofa\""), Some(p(2, 2, 2)));
    }
}
