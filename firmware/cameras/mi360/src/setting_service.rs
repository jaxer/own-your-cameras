//! Confirmed hardware settings, bounded queued changes and deferred persistence.
//! The media owner supplies hardware and time; this module never publishes MQTT.
use crate::media::ControlResult;
use crate::settings::{self, Confirmed, Descriptor, Store};
use camera_transport::mqtt::{Domain, Entity};
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::time::Duration;

const DEADLINE: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(250);

pub trait Hardware {
    fn get(&mut self, control: u32) -> io::Result<i32>;
    fn set(&mut self, control: u32, value: i32) -> io::Result<ControlResult>;
    fn software_get(&self, slot: u16) -> io::Result<i32>;
    fn software_set(&mut self, slot: u16, value: i32) -> io::Result<()>;
}

struct Pending {
    control: u32,
    requested: i32,
    previous: i32,
    deadline: Duration,
    next_poll: Duration,
    rollback: bool,
}

pub struct Service {
    store: Store,
    confirmed: Confirmed,
    available: BTreeSet<u16>,
    actual: BTreeMap<u16, i32>,
    pending: BTreeMap<u16, Pending>,
    startup: BTreeMap<u16, i32>,
    updates: BTreeMap<&'static str, String>,
    seen_healthy: bool,
    next_refresh: Duration,
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn queued_channel(control: Option<u32>) -> Option<u32> {
    match control {
        Some(control @ (20 | 21 | 24..=27)) => Some(control & 1),
        _ => None,
    }
}

fn read(hw: &mut impl Hardware, descriptor: &Descriptor) -> io::Result<(i32, i32)> {
    let raw = match descriptor.hardware {
        Some(control) => hw.get(control)?,
        None => hw.software_get(descriptor.slot)?,
    };
    if descriptor.slot <= 1 && !(0..=3).contains(&raw) {
        return Err(io::Error::other("invalid raw orientation readback"));
    }
    let value = descriptor.decode(raw);
    if !descriptor.valid(value) {
        return Err(io::Error::other("hardware readback outside catalog bounds"));
    }
    Ok((raw, value))
}

impl Service {
    pub fn initialize(
        hw: &mut impl Hardware,
        store: Store,
        audio: bool,
        now: Duration,
    ) -> io::Result<Self> {
        // Store skips unknown/out-of-range catalog fields. Malformed JSON,
        // document/version errors and unsafe filesystem metadata fail startup.
        let saved = store.load()?;
        Ok(Self::from_saved(hw, store, audio, now, saved))
    }

    fn from_saved(
        hw: &mut impl Hardware,
        store: Store,
        audio: bool,
        now: Duration,
        saved: BTreeMap<u16, i32>,
    ) -> Self {
        let mut service = Self {
            store,
            confirmed: Confirmed::default(),
            available: BTreeSet::new(),
            actual: BTreeMap::new(),
            pending: BTreeMap::new(),
            startup: BTreeMap::new(),
            updates: BTreeMap::new(),
            seen_healthy: false,
            next_refresh: now,
        };
        // One shared HV snapshot keeps the two logical axes coherent.
        for d in settings::CATALOG {
            if d.audio && !audio || d.slot == 1 {
                continue;
            }
            if let Ok((raw, value)) = read(hw, d) {
                service.available.insert(d.slot);
                service.confirmed.observe(d.slot, value);
                if d.slot == 0 {
                    service.available.insert(1);
                    service
                        .confirmed
                        .observe(1, settings::descriptor(1).unwrap().decode(raw));
                }
                service.observe(d, raw);
            }
        }
        service.status(format!("ready: {} controls", service.available.len()));
        // Default is physical stock HV3, independently of discovery publication.
        // Missing saved axes become logical OFF; combine both into one write.
        if service.available.contains(&0) {
            let mirror = saved
                .get(&0)
                .copied()
                .filter(|v| (0..=1).contains(v))
                .unwrap_or(0);
            let flip = saved
                .get(&1)
                .copied()
                .filter(|v| (0..=1).contains(v))
                .unwrap_or(0);
            let raw = settings::STOCK_HV ^ mirror ^ (flip << 1);
            let _ = service.apply(0, mirror, Some(raw), hw, now);
        }
        for (slot, value) in saved {
            if slot > 1
                && service.available.contains(&slot)
                && let Err(error) = service.request(slot, value, hw, now)
                && error.kind() == io::ErrorKind::WouldBlock
            {
                service.startup.insert(slot, value);
            }
        }
        service
    }

    pub fn entities(&self) -> Vec<Entity> {
        let mut result: Vec<_> = settings::CATALOG
            .iter()
            .filter(|d| self.available.contains(&d.slot))
            .map(Descriptor::entity)
            .collect();
        result.push(Entity {
            id: "settings".into(),
            name: "Settings status".into(),
            domain: Domain::Sensor,
            minimum: 0,
            maximum: 0,
            options: Vec::new(),
            unit: None,
            diagnostic: true,
            control_slot: None,
        });
        result
    }

    fn status(&mut self, message: String) {
        self.updates.insert("settings", message);
    }

    fn observe(&mut self, d: &Descriptor, raw: i32) {
        let slots: &[u16] = if d.slot <= 1 {
            &[0, 1]
        } else {
            std::slice::from_ref(&d.slot)
        };
        for slot in slots {
            if !self.available.contains(slot) {
                continue;
            }
            let descriptor = settings::descriptor(*slot).unwrap();
            let value = descriptor.decode(raw);
            if self.actual.insert(*slot, value) != Some(value) {
                self.updates.insert(descriptor.key, descriptor.text(value));
            }
        }
    }

    fn accept(&mut self, d: &Descriptor, raw: i32, now: Duration) {
        self.observe(d, raw);
        if d.slot <= 1 {
            for slot in 0..=1 {
                self.confirmed
                    .applied(slot, settings::descriptor(slot).unwrap().decode(raw), now);
            }
        } else {
            self.confirmed.applied(d.slot, d.decode(raw), now);
        }
    }

    fn rejected_readback(&mut self, d: &Descriptor, hw: &mut impl Hardware) {
        // Real changed state is publishable even after a failed mutation. It is
        // deliberately separate from the persistable confirmed snapshot.
        if let Ok((raw, _)) = read(hw, d) {
            self.observe(d, raw);
        }
    }

    fn refresh_gop(&mut self, control: u32, hw: &mut impl Hardware) {
        let Some(d) = settings::CATALOG
            .iter()
            .find(|d| d.hardware == Some(control))
        else {
            return;
        };
        if !self.available.contains(&d.slot) {
            return;
        }
        if let Ok((raw, _)) = read(hw, d) {
            self.observe(d, raw);
        } else if self.actual.remove(&d.slot).is_some() {
            // An unsupported GOP record or nonintegral GOP/FPS duration has no
            // scalar value. Retaining an old number would misrepresent readback.
            self.updates.insert(d.key, "unknown".into());
        }
    }

    fn refresh_after_fps(&mut self, control: Option<u32>, hw: &mut impl Hardware) {
        if let Some(control @ (20 | 21)) = control {
            self.refresh_gop(control + 4, hw);
        }
    }

    pub fn request(
        &mut self,
        slot: u16,
        value: i32,
        hw: &mut impl Hardware,
        now: Duration,
    ) -> io::Result<()> {
        if settings::descriptor(slot).is_some_and(|d| d.valid(value)) {
            // An explicit command supersedes an older deferred startup value.
            self.startup.remove(&slot);
        }
        self.apply(slot, value, None, hw, now)
    }

    fn apply(
        &mut self,
        slot: u16,
        value: i32,
        forced_raw: Option<i32>,
        hw: &mut impl Hardware,
        now: Duration,
    ) -> io::Result<()> {
        let d = settings::descriptor(slot).ok_or_else(|| invalid("unknown setting slot"))?;
        if !self.available.contains(&slot) || !d.valid(value) {
            self.status(format!("rejected: {} unavailable or out of range", d.key));
            return Err(invalid("setting unavailable or out of range"));
        }
        let channel = queued_channel(d.hardware);
        if self.pending.contains_key(&slot)
            || channel.is_some_and(|channel| {
                self.pending
                    .values()
                    .any(|p| queued_channel(Some(p.control)) == Some(channel))
            })
        {
            self.status(format!("busy: {} pending", d.key));
            return Err(io::Error::new(io::ErrorKind::WouldBlock, "setting pending"));
        }
        let (previous, _) = read(hw, d).inspect_err(|_| {
            self.status(format!("rejected: {} readback failed", d.key));
        })?;
        self.observe(d, previous);
        let target = forced_raw.unwrap_or_else(|| d.encode(value, previous));
        if previous == target {
            self.accept(d, previous, now);
            self.refresh_after_fps(d.hardware, hw);
            return Ok(());
        }
        let result = match d.hardware {
            Some(control) => hw.set(control, target),
            None => hw
                .software_set(slot, target)
                .map(|()| ControlResult::Applied),
        };
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.rejected_readback(d, hw);
                self.status(format!("rejected: {} hardware error", d.key));
                return Err(error);
            }
        };
        let after = read(hw, d);
        if let Ok((raw, _)) = after {
            self.observe(d, raw);
            if raw == target && result != ControlResult::Unverified {
                self.accept(d, raw, now);
                self.refresh_after_fps(d.hardware, hw);
                self.status(format!("applied: {}", d.key));
                return Ok(());
            }
        }
        // Exact queued FPS/GOP/multiplier capabilities have bounded rollback.
        if queued_channel(d.hardware).is_some() && result != ControlResult::Unverified {
            self.pending.insert(
                slot,
                Pending {
                    control: d.hardware.unwrap(),
                    requested: target,
                    previous,
                    deadline: now.saturating_add(DEADLINE),
                    next_poll: now.saturating_add(POLL),
                    rollback: false,
                },
            );
            self.status(format!("pending: {}", d.key));
            return Ok(());
        }
        self.status(format!("rejected: {} no matching readback", d.key));
        Err(io::Error::other(
            "setting lacks matching confirmed readback",
        ))
    }

    pub fn tick(&mut self, hw: &mut impl Hardware, now: Duration, healthy: bool) -> io::Result<()> {
        self.seen_healthy |= healthy;
        let due: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, p)| now >= p.next_poll || now >= p.deadline)
            .map(|(slot, _)| *slot)
            .collect();
        for slot in due {
            let mut p = self.pending.remove(&slot).unwrap();
            let d = settings::descriptor(slot).unwrap();
            let actual = read(hw, d).ok().map(|(raw, _)| raw);
            if let Some(raw) = actual {
                self.observe(d, raw);
                if raw == p.requested && now <= p.deadline {
                    if !p.rollback {
                        self.accept(d, raw, now);
                    }
                    self.refresh_after_fps(d.hardware, hw);
                    self.status(format!(
                        "{}: {}",
                        if p.rollback { "rolled back" } else { "applied" },
                        d.key
                    ));
                    continue;
                }
            }
            if now < p.deadline {
                p.next_poll = now.saturating_add(POLL);
                self.pending.insert(slot, p);
                continue;
            }
            if p.rollback {
                self.status(format!("unconfirmed rollback: {}", d.key));
                continue;
            }
            // One rollback command, then at most five further seconds of reads.
            // No retry commands and no persistence of the failed target.
            match hw.set(p.control, p.previous) {
                Ok(result) if result != ControlResult::Unverified => {
                    p.requested = p.previous;
                    p.rollback = true;
                    p.deadline = now.saturating_add(DEADLINE);
                    p.next_poll = now;
                    self.pending.insert(slot, p);
                    self.status(format!("rolling back: {}", d.key));
                }
                _ => {
                    self.rejected_readback(d, hw);
                    self.status(format!("rollback failed: {}", d.key));
                }
            }
        }
        // FPS and GOP are coupled inside the SDK worker. A one-second refresh
        // also catches the interval between its FPS and GOP updates, without
        // persisting a transient or failed requested duration.
        if now >= self.next_refresh {
            self.next_refresh = now.saturating_add(Duration::from_secs(1));
            self.refresh_gop(24, hw);
            self.refresh_gop(25, hw);
            self.refresh_gop(26, hw);
            self.refresh_gop(27, hw);
        }
        let ready: Vec<_> = self
            .startup
            .iter()
            .filter_map(|(slot, value)| {
                let channel = queued_channel(settings::descriptor(*slot)?.hardware);
                let blocked = channel.is_some_and(|channel| {
                    self.pending
                        .values()
                        .any(|p| queued_channel(Some(p.control)) == Some(channel))
                });
                (!blocked).then_some((*slot, *value))
            })
            .collect();
        for (slot, value) in ready {
            self.startup.remove(&slot);
            if let Err(error) = self.request(slot, value, hw, now)
                && error.kind() == io::ErrorKind::WouldBlock
            {
                self.startup.insert(slot, value);
            }
        }
        if healthy && self.confirmed.due(now) {
            self.flush()?;
        }
        Ok(())
    }

    pub fn take_updates(&mut self) -> Vec<(&'static str, String)> {
        std::mem::take(&mut self.updates).into_iter().collect()
    }

    pub fn flush(&mut self) -> io::Result<()> {
        if self.seen_healthy && self.confirmed.dirty() {
            if let Err(error) = self.store.save(&self.confirmed) {
                self.status("save failed: confirmed settings remain dirty".into());
                return Err(error);
            }
            self.confirmed.saved();
            self.status("saved".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        values: BTreeMap<u32, i32>,
        writes: Vec<(u32, i32)>,
        queued: bool,
        fail_set: bool,
        fail_get: bool,
        unverified: bool,
        speed: i32,
        software: BTreeMap<u16, i32>,
    }
    impl Fake {
        fn new() -> Self {
            Self {
                values: BTreeMap::from([
                    (0, 3),
                    (1, 128),
                    (20, 20),
                    (24, 1),
                    (26, 3),
                    (27, 3),
                    (10, 55),
                ]),
                speed: 300,
                software: BTreeMap::from([
                    (settings::START_SPEED, 100),
                    (settings::RAMP_UP, 250),
                    (settings::RAMP_DOWN, 150),
                ]),
                ..Self::default()
            }
        }
    }
    impl Hardware for Fake {
        fn get(&mut self, control: u32) -> io::Result<i32> {
            if self.fail_get {
                return Err(io::Error::other("fixture read failure"));
            }
            self.values
                .get(&control)
                .copied()
                .ok_or_else(|| io::Error::new(io::ErrorKind::Unsupported, "fixture unavailable"))
        }
        fn set(&mut self, control: u32, value: i32) -> io::Result<ControlResult> {
            self.writes.push((control, value));
            if !self.queued {
                self.values.insert(control, value);
            }
            if self.fail_set {
                return Err(io::Error::other("fixture mutation failed after write"));
            }
            Ok(if self.unverified {
                ControlResult::Unverified
            } else if self.queued {
                ControlResult::Pending
            } else {
                ControlResult::Applied
            })
        }
        fn software_get(&self, slot: u16) -> io::Result<i32> {
            if slot == settings::SPEED {
                return Ok(self.speed);
            }
            self.software
                .get(&slot)
                .copied()
                .ok_or_else(|| invalid("unknown software setting"))
        }
        fn software_set(&mut self, slot: u16, value: i32) -> io::Result<()> {
            if slot == settings::SPEED {
                self.speed = value;
            } else {
                *self
                    .software
                    .get_mut(&slot)
                    .ok_or_else(|| invalid("unknown software setting"))? = value;
            }
            Ok(())
        }
    }
    fn service(hw: &mut Fake, saved: BTreeMap<u16, i32>, audio: bool) -> Service {
        // No filesystem calls: a deliberately unusable store catches accidental
        // saves; persistence itself is covered by settings::Store tests.
        Service::from_saved(
            hw,
            Store::new("/nonexistent/mi360-isolated-settings/settings.json"),
            audio,
            Duration::ZERO,
            saved,
        )
    }
    fn at(seconds: u64) -> Duration {
        Duration::from_secs(seconds)
    }

    #[test]
    fn motion_settings_restore_publish_and_persist_only_valid_accepted_values() {
        let mut hw = Fake::new();
        let saved = BTreeMap::from([
            (settings::START_SPEED, 300),
            (settings::RAMP_UP, 800),
            (settings::RAMP_DOWN, 0),
        ]);
        let mut s = service(&mut hw, saved.clone(), false);
        for (slot, value) in saved {
            let d = settings::descriptor(slot).unwrap();
            assert_eq!(s.confirmed.get(slot), Some(value));
            assert_eq!(hw.software_get(slot).unwrap(), value);
            let e = s.entities().into_iter().find(|e| e.id == d.key).unwrap();
            assert_eq!(e.control_slot, Some(slot));
        }
        s.request(settings::RAMP_DOWN, 3000, &mut hw, at(1))
            .unwrap();
        assert_eq!(s.confirmed.get(settings::RAMP_DOWN), Some(3000));
        for (slot, bad) in [
            (settings::START_SPEED, 99),
            (settings::RAMP_UP, -1),
            (settings::RAMP_DOWN, 3001),
        ] {
            let before = hw.software_get(slot).unwrap();
            assert!(s.request(slot, bad, &mut hw, at(2)).is_err());
            assert_eq!(hw.software_get(slot).unwrap(), before);
        }
    }
    #[test]
    fn capabilities_are_probed_and_audio_is_gated() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        let ids: Vec<_> = s.entities().into_iter().map(|e| e.id).collect();
        assert!(ids.contains(&"brightness".into()) && ids.contains(&"mirror".into()));
        assert!(!ids.contains(&"contrast".into()) && !ids.contains(&"mic_volume".into()));
        let status = s
            .entities()
            .into_iter()
            .find(|e| e.id == "settings")
            .unwrap();
        assert!(status.diagnostic && status.control_slot.is_none());
        assert!(s.request(8, 60, &mut hw, at(1)).is_err());
        assert!(
            service(&mut hw, BTreeMap::new(), true)
                .available
                .contains(&8)
        );
    }
    #[test]
    fn white_balance_presets_encode_sdk_modes_and_persist_logical_indices_only() {
        let mut hw = Fake::new();
        hw.values.insert(35, 2); // SDK daylight mode maps to select index1.
        let mut s = service(&mut hw, BTreeMap::from([(23, 2)]), false);
        assert_eq!(hw.values[&35], 3); // saved select index2 maps to SDK cloudy3
        assert_eq!(s.actual.get(&23), Some(&2));
        assert_eq!(s.confirmed.get(23), Some(2));
        s.request(23, 7, &mut hw, at(1)).unwrap();
        assert_eq!(hw.values[&35], 8);
        assert_eq!(s.confirmed.get(23), Some(7));
        hw.values.insert(35, 1); // Manual gains are deliberately not restored.
        let s = service(&mut hw, BTreeMap::new(), false);
        assert!(!s.available.contains(&23));
    }
    #[test]
    fn startup_defaults_and_saved_orientation_are_combined_once() {
        let mut hw = Fake::new();
        hw.values.insert(0, 0);
        let mut s = service(&mut hw, BTreeMap::new(), false);
        assert_eq!(hw.writes, vec![(0, 3)]);
        assert_eq!(s.actual.get(&0), Some(&0));
        s.request(0, 1, &mut hw, at(1)).unwrap();
        assert_eq!(hw.values[&0], 2);
        s.request(1, 1, &mut hw, at(2)).unwrap();
        assert_eq!(hw.values[&0], 0);
        let mut hw = Fake::new();
        let s = service(&mut hw, BTreeMap::from([(1, 1)]), false);
        assert_eq!(hw.writes, vec![(0, 1)]);
        assert_eq!(s.actual.get(&0), Some(&0));
        assert_eq!(s.actual.get(&1), Some(&1));
    }
    #[test]
    fn failed_mutation_reports_actual_without_persisting_target() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        s.take_updates();
        hw.fail_set = true;
        assert!(s.request(2, 130, &mut hw, at(1)).is_err());
        assert_eq!(s.actual.get(&2), Some(&130));
        assert_eq!(s.confirmed.get(2), Some(128));
        assert!(!s.confirmed.dirty());
        assert!(s.take_updates().contains(&("brightness", "130".into())));
    }
    #[test]
    fn pending_target_is_not_state_or_persistent_until_active_readback() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        hw.queued = true;
        s.request(10, 10, &mut hw, at(1)).unwrap();
        assert_eq!(s.actual.get(&10), Some(&20));
        assert_eq!(s.confirmed.get(10), Some(20));
        assert!(!s.confirmed.dirty());
        assert!(s.request(10, 5, &mut hw, at(2)).is_err());
        hw.values.insert(20, 10);
        s.tick(&mut hw, at(2), false).unwrap();
        assert!(s.pending.is_empty());
        assert_eq!(s.confirmed.get(10), Some(10));
        assert!(!s.confirmed.due(at(11)));
        assert!(s.confirmed.due(at(12)));
    }
    #[test]
    fn fps_and_gop_pending_changes_exclude_each_other_per_channel() {
        let mut hw = Fake::new();
        hw.values.insert(21, 20);
        hw.values.insert(25, 1);
        let mut s = service(&mut hw, BTreeMap::new(), false);
        hw.queued = true;
        s.request(10, 10, &mut hw, at(1)).unwrap();
        assert!(s.request(14, 2, &mut hw, at(1)).is_err());
        // The other channel is independent, including the reverse conflict.
        s.request(15, 2, &mut hw, at(1)).unwrap();
        assert!(s.request(11, 10, &mut hw, at(1)).is_err());
        assert_eq!(hw.writes, vec![(20, 10), (25, 2)]);
        hw.values.insert(20, 10);
        hw.values.insert(24, 2); // real secondary readback, never a desired substitute
        s.tick(&mut hw, at(2), false).unwrap();
        assert_eq!(s.actual.get(&14), Some(&2));
        assert_eq!(s.confirmed.get(14), Some(1));
        s.request(14, 3, &mut hw, at(2)).unwrap();
        assert_eq!(hw.writes.last(), Some(&(24, 3)));
    }
    #[test]
    fn multiplier_pending_excludes_all_same_channel_graph_controls() {
        for first in [20, 24, 26] {
            let mut hw = Fake::new();
            hw.values.insert(21, 20);
            hw.values.insert(25, 1);
            let mut s = service(&mut hw, BTreeMap::new(), false);
            hw.queued = true;
            let setting = |control| {
                settings::CATALOG
                    .iter()
                    .find(|d| d.hardware == Some(control))
                    .unwrap()
                    .slot
            };
            let target = |control| {
                if control <= 21 {
                    10
                } else if control <= 25 {
                    2
                } else {
                    1
                }
            };
            s.request(setting(first), target(first), &mut hw, at(1))
                .unwrap();
            for second in [20, 24, 26] {
                assert_eq!(
                    s.request(setting(second), target(second), &mut hw, at(1))
                        .unwrap_err()
                        .kind(),
                    io::ErrorKind::WouldBlock
                );
            }
            s.request(setting(first + 1), target(first + 1), &mut hw, at(1))
                .unwrap();
            assert_eq!(hw.writes.len(), 2);
        }
    }
    #[test]
    fn saved_coupled_controls_wait_for_confirmation_and_new_commands_supersede_them() {
        let mut hw = Fake::new();
        hw.queued = true;
        let mut s = service(&mut hw, BTreeMap::from([(10, 10), (14, 2), (16, 1)]), false);
        assert_eq!(hw.writes, vec![(20, 10)]);
        assert_eq!(s.startup, BTreeMap::from([(14, 2), (16, 1)]));
        hw.values.insert(20, 10);
        s.tick(&mut hw, at(1), false).unwrap();
        assert_eq!(hw.writes, vec![(20, 10), (24, 2)]);
        assert_eq!(s.startup, BTreeMap::from([(16, 1)]));
        hw.values.insert(24, 2);
        s.tick(&mut hw, at(2), false).unwrap();
        assert_eq!(hw.writes.last(), Some(&(26, 1)));
        hw.values.insert(26, 1);
        s.tick(&mut hw, at(3), false).unwrap();
        assert_eq!(s.confirmed.get(16), Some(1));
        assert!(s.startup.is_empty() && s.pending.is_empty());

        let mut hw = Fake::new();
        hw.queued = true;
        let mut s = service(&mut hw, BTreeMap::from([(10, 10), (16, 1)]), false);
        assert!(s.request(16, 2, &mut hw, at(1)).is_err());
        assert!(s.startup.is_empty());
        hw.values.insert(20, 10);
        s.tick(&mut hw, at(2), false).unwrap();
        assert_eq!(hw.writes, vec![(20, 10)]);
    }
    #[test]
    fn gop_refresh_marks_failed_readback_unknown_and_recovers_without_persisting() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        s.take_updates();
        hw.values.remove(&24);
        s.tick(&mut hw, at(1), false).unwrap();
        let key = settings::descriptor(14).unwrap().key;
        assert!(s.take_updates().contains(&(key, "unknown".into())));
        assert!(!s.actual.contains_key(&14));
        hw.values.insert(24, 2);
        s.tick(&mut hw, at(2), false).unwrap();
        assert!(s.take_updates().contains(&(key, "2".into())));
        assert_eq!(s.confirmed.get(14), Some(1));
        assert!(!s.confirmed.dirty());
    }
    #[test]
    fn timeout_rolls_back_once_and_stops_after_a_second_bounded_window() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        hw.queued = true;
        s.request(14, 2, &mut hw, at(1)).unwrap();
        hw.values.insert(24, 3);
        s.tick(&mut hw, at(6), false).unwrap();
        assert_eq!(hw.writes, vec![(24, 2), (24, 1)]);
        s.tick(&mut hw, at(11), false).unwrap();
        assert!(s.pending.is_empty());
        s.tick(&mut hw, at(100), false).unwrap();
        assert_eq!(hw.writes.len(), 2);
        assert_eq!(s.actual.get(&14), Some(&3));
        assert_eq!(s.confirmed.get(14), Some(1));
        assert!(!s.confirmed.dirty());
    }
    #[test]
    fn rollback_readback_can_finish_without_saving_failed_target() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        hw.queued = true;
        s.request(10, 10, &mut hw, at(1)).unwrap();
        s.tick(&mut hw, at(6), false).unwrap();
        s.tick(&mut hw, at(6), false).unwrap();
        assert!(s.pending.is_empty() && !s.confirmed.dirty());
        assert_eq!(s.confirmed.get(10), Some(20));
    }
    #[test]
    fn read_failures_unverified_and_invalid_requests_never_confirm() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        assert!(s.request(2, 256, &mut hw, at(1)).is_err());
        assert!(hw.writes.is_empty());
        hw.fail_get = true;
        assert!(s.request(2, 130, &mut hw, at(1)).is_err());
        assert!(hw.writes.is_empty());
        hw.fail_get = false;
        hw.unverified = true;
        assert!(s.request(2, 130, &mut hw, at(1)).is_err());
        assert_eq!(s.confirmed.get(2), Some(128));
        assert!(!s.confirmed.dirty());
    }
    #[test]
    fn late_target_and_failed_rollback_never_confirm_or_retry() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        hw.queued = true;
        s.request(10, 10, &mut hw, at(1)).unwrap();
        hw.values.insert(20, 10); // first observed after the five-second budget
        hw.fail_set = true;
        s.tick(&mut hw, at(7), false).unwrap();
        assert!(s.pending.is_empty());
        assert_eq!(hw.writes, vec![(20, 10), (20, 20)]);
        assert_eq!(s.actual.get(&10), Some(&10));
        assert_eq!(s.confirmed.get(10), Some(20));
        assert!(!s.confirmed.dirty());
        s.tick(&mut hw, at(100), false).unwrap();
        assert_eq!(hw.writes.len(), 2);
    }
    #[test]
    fn failed_pending_reads_still_have_a_bounded_rollback_lifecycle() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        hw.queued = true;
        s.request(10, 10, &mut hw, at(1)).unwrap();
        hw.fail_get = true;
        s.tick(&mut hw, at(6), false).unwrap();
        s.tick(&mut hw, at(11), false).unwrap();
        assert!(s.pending.is_empty());
        assert_eq!(hw.writes, vec![(20, 10), (20, 20)]);
        assert!(!s.confirmed.dirty());
    }
    #[test]
    fn command_bursts_coalesce_updates_by_catalog_key() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        s.take_updates();
        for value in 0..=255 {
            s.request(2, value, &mut hw, at(1)).unwrap();
        }
        assert_eq!(s.updates.len(), 2);
        let updates = s.take_updates();
        assert!(updates.contains(&("brightness", "255".into())));
        assert!(s.take_updates().is_empty());
    }
    #[test]
    fn save_failure_preserves_first_dirty_deadline_and_unhealthy_defers_io() {
        let mut hw = Fake::new();
        let mut s = service(&mut hw, BTreeMap::new(), false);
        s.request(2, 129, &mut hw, at(1)).unwrap();
        s.request(7, 400, &mut hw, at(9)).unwrap();
        assert!(!s.confirmed.due(at(10)));
        assert!(s.confirmed.due(at(11)));
        s.tick(&mut hw, at(11), false).unwrap();
        assert!(s.confirmed.dirty());
        s.flush().unwrap();
        assert!(s.confirmed.dirty()); // probation never writes
        assert!(s.tick(&mut hw, at(11), true).is_err());
        assert!(s.confirmed.due(at(11)));
        assert_eq!(s.confirmed.get(7), Some(400));
    }
}
