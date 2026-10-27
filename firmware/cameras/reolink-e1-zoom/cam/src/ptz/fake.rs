//! Host fake of the MS32006: a register file plus a tiny model of the two motors and their end stops.
//!
//! A kick (`reg 9`) makes the axis report busy for [`BUSY_POLLS`] status reads, after which the move is
//! applied to the mechanical position, clamped to `0..=max` like a head driven into its stop.

use std::time::Duration;

use super::{Axis, Bus, Error, PAN_MAX, TILT_MAX};

const BUSY_POLLS: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    pub axis: Axis,
    pub steps: u32,
    /// Step-rate divisor programmed when the move was kicked.
    pub divisor: u32,
}

impl Move {
    /// The rate the divisor stands for (`15 MHz / (divisor << 7)`).
    pub fn pps(&self) -> u32 {
        15_000_000 / (self.divisor.max(1) << 7)
    }
}

pub struct FakeBus {
    regs: [u8; 128],
    /// Mechanical position in vendor coordinates (pan x, tilt y).
    mech: [u32; 2],
    /// Status reads left before the running move completes, per axis.
    busy: [u32; 2],
    /// Move waiting to be applied when its busy polls run out: signed distance in vendor coordinates.
    pending: [i64; 2],
    moves: Vec<Move>,
    kicks: Vec<Axis>,
    ctrl_writes: Vec<u8>,
    sleeps: Vec<Duration>,
    stuck: bool,
    failing: bool,
}

impl FakeBus {
    pub fn new(pan: u32, tilt: u32) -> Self {
        Self {
            regs: [0; 128],
            mech: [pan, tilt],
            busy: [0; 2],
            pending: [0; 2],
            moves: Vec::new(),
            kicks: Vec::new(),
            ctrl_writes: Vec::new(),
            sleeps: Vec::new(),
            stuck: false,
            failing: false,
        }
    }

    /// Every status read reports busy and no move ever completes.
    pub fn stuck_busy(mut self) -> Self {
        self.stuck = true;
        self
    }

    /// Every register access fails.
    pub fn failing(mut self) -> Self {
        self.failing = true;
        self
    }

    pub fn regs(&self) -> &[u8; 128] {
        &self.regs
    }

    pub fn mech(&self) -> (u32, u32) {
        (self.mech[0], self.mech[1])
    }

    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    pub fn kicks(&self) -> &[Axis] {
        &self.kicks
    }

    pub fn sleeps(&self) -> &[Duration] {
        &self.sleeps
    }

    /// Values written to the control register, in order.
    pub fn writes_to(&self, reg: u8) -> &[u8] {
        assert_eq!(reg, 0, "only the control register is logged");
        &self.ctrl_writes
    }

    fn kick(&mut self, axis: Axis) {
        let i = axis.index();
        let bank = axis.bank() as usize;
        let steps = u32::from(self.regs[bank + 3]) | u32::from(self.regs[bank + 4] & 0x0f) << 8;
        let dir_one = self.regs[bank + 4] & 0x40 != 0;
        let divisor = u32::from(self.regs[bank + 1]) | u32::from(self.regs[bank + 2] & 0x3f) << 8;
        self.moves.push(Move {
            axis,
            steps,
            divisor,
        });
        self.kicks.push(axis);
        if self.regs[0] & axis.stop_bit() != 0 {
            return;
        }
        let towards_positive = (axis.positive() == super::Dir::One) == dir_one;
        let signed = i64::from(steps);
        self.pending[i] = if towards_positive { signed } else { -signed };
        self.busy[i] = BUSY_POLLS;
    }

    fn finish(&mut self, i: usize) {
        let max = [PAN_MAX, TILT_MAX][i];
        let to = i64::from(self.mech[i]) + self.pending[i];
        self.mech[i] = to.clamp(0, i64::from(max)) as u32;
        self.pending[i] = 0;
    }

    fn status(&mut self, axis: Axis) -> [u8; 2] {
        let i = axis.index();
        if self.stuck {
            return [0x90, 0];
        }
        if self.busy[i] > 0 {
            self.busy[i] -= 1;
            if self.busy[i] == 0 {
                self.finish(i);
            } else {
                return [0x90, 0];
            }
        }
        [0x80, 0]
    }
}

impl Bus for FakeBus {
    fn read(&mut self, reg: u8, buf: &mut [u8]) -> Result<(), Error> {
        if self.failing {
            return Err(Error::Bus("fake read failure".into()));
        }
        for (o, slot) in buf.iter_mut().enumerate() {
            let r = reg.wrapping_add(o as u8);
            if (r == Axis::Pan.status_reg() || r == Axis::Tilt.status_reg()) && o == 0 {
                let axis = if r == Axis::Pan.status_reg() {
                    Axis::Pan
                } else {
                    Axis::Tilt
                };
                let s = self.status(axis);
                *slot = s[0];
                if let Some(next) = buf.get_mut(1) {
                    *next = s[1];
                }
                return Ok(());
            }
            *slot = self.regs[usize::from(r & 0x7f)];
        }
        Ok(())
    }

    fn write(&mut self, reg: u8, data: &[u8]) -> Result<(), Error> {
        if self.failing {
            return Err(Error::Bus("fake write failure".into()));
        }
        for (o, v) in data.iter().enumerate() {
            let r = usize::from(reg.wrapping_add(o as u8) & 0x7f);
            self.regs[r] = *v;
            match r {
                0 => {
                    self.ctrl_writes.push(*v);
                    for axis in Axis::ALL {
                        // a set stop bit aborts a running move on that axis
                        if *v & axis.stop_bit() != 0 {
                            self.busy[axis.index()] = 0;
                            self.pending[axis.index()] = 0;
                        }
                    }
                }
                9 => {
                    for axis in Axis::ALL {
                        if *v & axis.kick_bit() != 0 {
                            self.kick(axis);
                        }
                    }
                    // the kick bits self-clear on the real chip (register 9 reads 0 after a move)
                    self.regs[9] = 0;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn sleep(&mut self, d: Duration) {
        self.sleeps.push(d);
    }
}
