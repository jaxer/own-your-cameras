//! Presets from the vendor's `/mnt/para/0_ptz.cfg`: one `<channel .../>` element whose attributes hold
//! `bval<N>` (slot in use), `pos<N>_x` (pan steps) and `pos<N>_y` (tilt steps).

use crate::ptz::{Axis, Position};

pub const PATH: &str = "/mnt/para/0_ptz.cfg";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("preset {0} is not stored in the config")]
    Missing(u32),
    #[error("preset {0} is not set (bval{0}=\"0\")")]
    Unset(u32),
    #[error("preset {n} ({pan}, {tilt}) is outside the travel")]
    OutOfRange { n: u32, pan: i64, tilt: i64 },
}

/// The integer value of ` name="..."` in `text`; the leading space keeps `pos1_x` from matching `pos11_x`.
fn attr(text: &str, name: &str) -> Option<i64> {
    let key = format!(" {name}=\"");
    let start = text.find(&key)? + key.len();
    let end = text[start..].find('"')? + start;
    text[start..end].trim().parse().ok()
}

/// Preset `n` of the config text (`n = 0` is the "Default" slot).
pub fn preset(cfg: &str, n: u32) -> Result<Position, Error> {
    let (Some(x), Some(y)) = (
        attr(cfg, &format!("pos{n}_x")),
        attr(cfg, &format!("pos{n}_y")),
    ) else {
        return Err(Error::Missing(n));
    };
    if attr(cfg, &format!("bval{n}")) != Some(1) {
        return Err(Error::Unset(n));
    }
    let fits = |v: i64, axis: Axis| u32::try_from(v).ok().filter(|v| *v <= axis.max());
    match (fits(x, Axis::Pan), fits(y, Axis::Tilt)) {
        (Some(pan), Some(tilt)) => Ok(Position { pan, tilt }),
        _ => Err(Error::OutOfRange { n, pan: x, tilt: y }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // trimmed from a camera's real file
    const CFG: &str = r#"<ptz_config_t>
    <channel addr="1" default_x="6195" default_y="650" n0="Default" bval0="1" pimg0="" pos0_x="3577" pos0_y="0" pos0_z="0" n1="pos1" bval1="1" pimg1="" pos1_x="3262" pos1_y="739" pos1_z="0" n2="pos2" bval2="1" pimg2="" pos2_x="6195" pos2_y="650" pos2_z="0" n3="pos4" bval3="0" pimg3="" pos3_x="-1" pos3_y="-1" pos3_z="-1" n10="pos11" bval10="1" pos10_x="9999" pos10_y="1" cc0_p0="-1" />
</ptz_config_t>"#;

    #[test]
    fn reads_the_vendor_presets() {
        assert_eq!(preset(CFG, 0), Ok(Position { pan: 3577, tilt: 0 }));
        assert_eq!(
            preset(CFG, 1),
            Ok(Position {
                pan: 3262,
                tilt: 739
            })
        );
        assert_eq!(
            preset(CFG, 2),
            Ok(Position {
                pan: 6195,
                tilt: 650
            })
        );
    }

    #[test]
    fn unset_slots_are_refused() {
        assert_eq!(preset(CFG, 3), Err(Error::Unset(3)));
    }

    #[test]
    fn missing_slots_are_refused() {
        assert_eq!(preset(CFG, 40), Err(Error::Missing(40)));
        assert_eq!(preset("", 1), Err(Error::Missing(1)));
    }

    #[test]
    fn slot_1_does_not_read_slot_10() {
        assert_eq!(preset(CFG, 1).unwrap().pan, 3262);
        assert_eq!(
            preset(CFG, 10),
            Err(Error::OutOfRange {
                n: 10,
                pan: 9999,
                tilt: 1
            })
        );
    }
}
