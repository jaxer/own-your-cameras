//! Preserve the single E1 channel's stock image settings at the sensor boundary.
use crate::zoom::{Error, Isp, WORDS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Orientation {
    mirror: bool,
    flip: bool,
}
impl Orientation {
    pub const fn new(mirror: bool, flip: bool) -> Self {
        Self { mirror, flip }
    }

    /// The picture is mirrored left-right relative to the stock mounting.
    pub const fn mirror(self) -> bool {
        self.mirror
    }
    /// The picture is flipped upside down relative to the stock mounting.
    pub const fn flip(self) -> bool {
        self.flip
    }

    /// Read the two boolean attributes from stock's bounded `0_isp.cfg` channel.
    pub fn from_vendor(text: &str) -> Result<Self, String> {
        let invalid = || "invalid stock mirror/flip settings".to_string();
        if text.len() > 16 * 1024 {
            return Err(invalid());
        }
        let (_, tail) = text.split_once("<channel ").ok_or_else(invalid)?;
        let (attrs, rest) = tail.split_once('>').ok_or_else(invalid)?;
        if rest.contains("<channel ") {
            return Err(invalid());
        }
        let flag = |name: &str| {
            let mut found = attrs.split_ascii_whitespace().filter_map(|a| {
                let (key, value) = a.split_once('=')?;
                (key == name).then_some(value.trim_end_matches('/'))
            });
            let value = match found.next() {
                Some("\"0\"") => false,
                Some("\"1\"") => true,
                _ => return Err(invalid()),
            };
            if found.next().is_some() {
                return Err(invalid());
            }
            Ok(value)
        };
        Ok(Self {
            mirror: flag("mirror")?,
            flip: flag("flip")?,
        })
    }
    /// ISP common item44: sensor ID, mirror, flip; not an HDAL output-direction mask.
    pub fn sensor_words(self) -> [u32; 3] {
        // Stock E1 Zoom dvr.xml: ipc_mirror=1, ipc_flip=0. User settings
        // are relative to that mounting correction, not raw sensor directions.
        [0, u32::from(!self.mirror), u32::from(self.flip)]
    }
    pub fn apply(self, isp: &mut impl Isp) -> Result<(), Error> {
        let mut words = [0; WORDS];
        words[..3].copy_from_slice(&self.sensor_words());
        isp.set(44, &words)?;
        // The kernel getter only writes enabled bits; a fresh zeroed buffer is required.
        let mut actual = [0; WORDS];
        isp.get(44, &mut actual)?;
        if actual[..3] != self.sensor_words() {
            return Err(Error::Ioctl("sensor orientation readback mismatch".into()));
        }
        Ok(())
    }
}
