//! Host fake of the `/dev/nvt_isp` lens items: two motors that report busy for [`BUSY_POLLS`] status reads after
//! a move, a focus window, the driver's focus-curve rows, and a VA (sharpness) reading that peaks at `sharp`, follows a
//! recorded focus curve, or replays a scripted sequence (a scene that changes over time).

use std::time::Duration;

use super::{Error, Isp, VA_CELLS, WORDS, cmd, item};

pub const BUSY_POLLS: u32 = 2;

/// One `set` the fake received: which item and the leading payload words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub item: u32,
    pub words: Vec<u32>,
}

pub struct FakeIsp {
    pub zoom: u32,
    pub focus: u32,
    /// What `MTR_GET_FOCUS_RANGE` answers (the window of the current zoom's row).
    pub focus_window: (u32, u32),
    /// Focus position where the VA peaks; `None` gives a flat VA (a scene without contrast).
    pub sharp: Option<u32>,
    /// VA as a function of focus, linear between recorded (focus, VA) points; overrides `sharp`.
    pub curve: Option<Vec<(u32, u64)>>,
    /// Percent the VA is scaled by (a darker or brighter scene).
    pub scale_pct: u64,
    /// VA reads answered from here first, one value per read (a scene changing over time).
    pub va_script: std::collections::VecDeque<u64>,
    /// VA reads so far.
    pub va_reads: u32,
    /// Focus-curve rows the driver holds: (zoom, focus min, focus max); unused slots read as 0xffff.
    pub rows: Vec<(u32, u32, u32)>,
    backlash: (u32, u32),
    busy: u32,
    sets: Vec<Set>,
    sleeps: Vec<Duration>,
    stuck: bool,
    /// Zoom speed as last set (`ZOOM_SPEED`).
    pub speed: u32,
    /// A press move runs: each status read advances the zoom 16 toward its end until released.
    pub pressing: Option<bool>,
    /// Sensor orientation (common item 44) as last set: sensor id, mirror, flip.
    pub orientation: Option<[u32; 3]>,
    /// Image-quality items as last set (leading words); a get echoes them.
    pub iq: std::collections::BTreeMap<u32, Vec<u32>>,
    /// IQ items whose get reports this instead of what was set (a driver that clamps or ignores).
    pub iq_readback: std::collections::BTreeMap<u32, Vec<u32>>,
}

impl FakeIsp {
    pub fn new(zoom: u32, focus: u32) -> Self {
        Self {
            zoom,
            focus,
            focus_window: (0, 1705),
            sharp: Some(600),
            curve: None,
            scale_pct: 100,
            va_script: Default::default(),
            va_reads: 0,
            rows: Vec::new(),
            backlash: (0, 0),
            busy: 0,
            sets: Vec::new(),
            sleeps: Vec::new(),
            stuck: false,
            speed: 80,
            pressing: None,
            orientation: None,
            iq: Default::default(),
            iq_readback: Default::default(),
        }
    }

    /// The motors report busy forever.
    pub fn stuck(mut self) -> Self {
        self.stuck = true;
        self
    }

    pub fn sets(&self) -> &[Set] {
        &self.sets
    }

    /// Forgets the sets so far (a test counts from here).
    pub fn clear_sets(&mut self) {
        self.sets.clear();
    }

    /// Focus targets in the order they were sent.
    pub fn focus_moves(&self) -> Vec<u32> {
        self.sets
            .iter()
            .filter(|s| s.item == item::FOCUS)
            .map(|s| s.words[1])
            .collect()
    }

    pub fn slept(&self) -> Duration {
        self.sleeps.iter().sum()
    }

    /// Sum of the VA cells at the current focus: a peak of 100 000 over a floor of 1 000.
    fn va(&mut self) -> u64 {
        self.va_reads += 1;
        if let Some(v) = self.va_script.pop_front() {
            return v;
        }
        let raw = match (&self.curve, self.sharp) {
            (Some(c), _) => interpolate(c, self.focus),
            (None, None) => 5_000,
            (None, Some(s)) => {
                let d = i64::from(self.focus) - i64::from(s);
                1_000 + (100_000 * 400 / (400 + d * d)) as u64
            }
        };
        raw * self.scale_pct / 100
    }
}

impl Isp for FakeIsp {
    fn get_iq(&mut self, it: u32, w: &mut [u32; WORDS]) -> Result<(), Error> {
        let src = self
            .iq_readback
            .get(&it)
            .or_else(|| self.iq.get(&it))
            .ok_or_else(|| Error::Ioctl(format!("fake: IQ item {it} never set")))?;
        w[..src.len()].copy_from_slice(src);
        Ok(())
    }
    fn set_iq(&mut self, it: u32, w: &[u32; WORDS]) -> Result<(), Error> {
        self.iq.insert(it, w[..16].to_vec());
        Ok(())
    }
    // AE items share the IQ store at item + 1000 so tests see one map.
    fn get_ae(&mut self, it: u32, w: &mut [u32; WORDS]) -> Result<(), Error> {
        self.get_iq(it + 1000, w)
    }
    fn set_ae(&mut self, it: u32, w: &[u32; WORDS]) -> Result<(), Error> {
        self.set_iq(it + 1000, w)
    }
    fn get(&mut self, it: u32, w: &mut [u32; WORDS]) -> Result<(), Error> {
        if it == 44 {
            let o = self
                .orientation
                .ok_or_else(|| Error::Ioctl("fake: orientation never set".into()))?;
            w[..3].copy_from_slice(&o);
            return Ok(());
        }
        match (it, w[0]) {
            (item::FOCUS | item::ZOOM, cmd::POSITION) => {
                let pos = if it == item::ZOOM {
                    self.zoom
                } else {
                    self.focus
                };
                // u16 position, u16 direction flag
                w[3] = pos | (1 << 16);
            }
            (item::FOCUS, cmd::RANGE) => (w[3], w[4]) = self.focus_window,
            (item::FOCUS, cmd::STATUS) => {
                if let Some(up) = self.pressing {
                    self.zoom = if up {
                        (self.zoom + 16).min(1024)
                    } else {
                        self.zoom.saturating_sub(16)
                    };
                    if self.zoom == 0 || self.zoom == 1024 {
                        self.pressing = None;
                    }
                }
                w[3] = u32::from(self.stuck || self.busy > 0 || self.pressing.is_some());
                self.busy = self.busy.saturating_sub(1);
            }
            (item::MISC, cmd::RANGE) => {
                let r = self.rows.get(w[1] as usize).copied();
                (w[18], w[19], w[20]) = r.unwrap_or((0xffff, 0xffff, 0xffff));
            }
            (item::MISC, 6) => (w[17], w[18]) = self.backlash,
            (item::VA, _) => {
                // spread over all cells so a reader that sums only some of them gets it wrong
                let va = u32::try_from(self.va()).unwrap_or(u32::MAX);
                let cells = &mut w[1..=VA_CELLS];
                cells.fill(va / VA_CELLS as u32);
                cells[0] += va % VA_CELLS as u32;
            }
            _ => return Err(Error::Ioctl(format!("fake: no get {it} cmd {}", w[0]))),
        }
        Ok(())
    }

    fn set(&mut self, it: u32, w: &[u32; WORDS]) -> Result<(), Error> {
        self.sets.push(Set {
            item: it,
            words: w[..6].to_vec(),
        });
        if it == 44 {
            self.orientation = Some([w[0], w[1], w[2]]);
            return Ok(());
        }
        match (it, w[0]) {
            (item::MISC, 1) => {
                self.rows
                    .resize(super::ROWS as usize, (0xffff, 0xffff, 0xffff));
                self.rows[w[1] as usize] = (w[2], w[3], w[4]);
                return Ok(());
            }
            (item::MISC, 4) => {
                self.backlash = (w[1], w[2]);
                return Ok(());
            }
            (item::ZOOM | item::FOCUS, cmd::INIT) => {}
            (item::ZOOM, cmd::ZOOM_GO) => self.zoom = w[1],
            (item::ZOOM, cmd::ZOOM_SPEED) => {
                self.speed = w[1];
                return Ok(());
            }
            (item::ZOOM, cmd::ZOOM_PRESS) => {
                self.pressing = Some(w[1] != 0);
                return Ok(());
            }
            (item::ZOOM, cmd::ZOOM_RELEASE) => {
                self.pressing = None;
                return Ok(());
            }
            (item::FOCUS, cmd::FOCUS_GO) => self.focus = w[1],
            _ => return Err(Error::Ioctl(format!("fake: no set {it} cmd {}", w[0]))),
        }
        self.busy = BUSY_POLLS;
        Ok(())
    }

    fn sleep(&mut self, d: Duration) {
        self.sleeps.push(d);
    }
}

/// VA at `focus` on a recorded curve: linear between neighbours, the end value beyond either end.
fn interpolate(curve: &[(u32, u64)], focus: u32) -> u64 {
    let Some(i) = curve.iter().position(|p| p.0 >= focus) else {
        return curve.last().map_or(0, |p| p.1);
    };
    if i == 0 || curve[i].0 == focus {
        return curve[i].1;
    }
    let ((f0, v0), (f1, v1)) = (curve[i - 1], curve[i]);
    let (span, t) = (i128::from(f1 - f0), i128::from(focus - f0));
    (i128::from(v0) + (i128::from(v1) - i128::from(v0)) * t / span) as u64
}
