//! One reading of the main network, worked through in small units.

use super::{
    HEAD_CHANNELS, HEADS, Head, HeadMaps, MAP_HEIGHT, MAP_WIDTH, PICTURE_HEIGHT, PICTURE_WIDTH,
    SpineNet,
};
use crate::grid::Grid;
use crate::layers::conv::Conv3x3;
use crate::layers::resample::upsample_twice;
use shapes::errors::{EngineError, ErrorCode};

/// About how many multiply-adds one unit of work holds. In WebAssembly on
/// a laptop that is a millisecond or two, so a unit never comes near the
/// browser's limit for a silent worker however slow the machine.
const UNIT_COST: usize = 16 << 20;

/// About how many values one unit of upsampling writes.
const UNIT_VALUES: usize = 1 << 19;

/// The maps a reading holds between units. Each is made when a step first
/// writes it and let go as soon as no later step reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    /// The network's input.
    Picture,
    /// A block's map between its two convolutions.
    Middle,
    /// The five encoder blocks' maps.
    E1,
    E2,
    E3,
    E4,
    E5,
    /// A map made twice as big for the decoder block above it.
    Doubled,
    /// A decoder block's map.
    Decoded,
}

const SLOTS: usize = 9;

/// One step of the reading, done in one or more units.
#[derive(Clone, Copy, Debug)]
enum Step {
    /// The picture's bytes become the network's input.
    Picture,
    /// One convolution, `conv` counting through [`SpineNet::convs`].
    Conv {
        conv: usize,
        from: &'static [Slot],
        to: Slot,
        /// The maps no later step reads.
        drop: &'static [Slot],
        /// The block this convolution completes, if any.
        finishes: Option<&'static str>,
    },
    /// A map made twice as big into [`Slot::Doubled`], then let go.
    Double { from: Slot },
    /// The three heads read [`Slot::Decoded`].
    Heads,
}

use Slot::{Decoded, Doubled, E1, E2, E3, E4, E5, Middle, Picture};

const fn conv(
    conv: usize,
    from: &'static [Slot],
    to: Slot,
    drop: &'static [Slot],
    finishes: Option<&'static str>,
) -> Step {
    Step::Conv {
        conv,
        from,
        to,
        drop,
        finishes,
    }
}

/// The whole reading, in order. Convolution `2k` and `2k + 1` are block
/// `k`'s first and second.
const STEPS: [Step; 21] = [
    Step::Picture,
    conv(0, &[Picture], Middle, &[Picture], None),
    conv(1, &[Middle], E1, &[Middle], Some("e1")),
    conv(2, &[E1], Middle, &[E1], None),
    conv(3, &[Middle], E2, &[Middle], Some("e2")),
    conv(4, &[E2], Middle, &[], None),
    conv(5, &[Middle], E3, &[Middle], Some("e3")),
    conv(6, &[E3], Middle, &[], None),
    conv(7, &[Middle], E4, &[Middle], Some("e4")),
    conv(8, &[E4], Middle, &[], None),
    conv(9, &[Middle], E5, &[Middle], Some("e5")),
    Step::Double { from: E5 },
    conv(10, &[Doubled, E4], Middle, &[Doubled, E4], None),
    conv(11, &[Middle], Decoded, &[Middle], Some("d4")),
    Step::Double { from: Decoded },
    conv(12, &[Doubled, E3], Middle, &[Doubled, E3], None),
    conv(13, &[Middle], Decoded, &[Middle], Some("d3")),
    Step::Double { from: Decoded },
    conv(14, &[Doubled, E2], Middle, &[Doubled, E2], None),
    conv(15, &[Middle], Decoded, &[Middle], Some("d2")),
    Step::Heads,
];

/// One step with its size worked out.
#[derive(Clone, Copy, Debug)]
pub(super) struct Planned {
    step: Step,
    /// Channels, rows and columns of what the step writes.
    shape: [usize; 3],
    /// Rows (convolutions and heads) or channels (upsampling) one unit
    /// covers.
    per_unit: usize,
    units: usize,
}

/// Works out every step's size and how it is cut into units. It depends
/// only on the network.
pub(super) fn plan(convs: &[Conv3x3], heads: &[Head]) -> Vec<Planned> {
    let mut shapes = [[0usize; 3]; SLOTS];
    let mut plan = Vec::with_capacity(STEPS.len());
    for step in STEPS {
        let planned = match step {
            Step::Picture => {
                shapes[Picture as usize] = [1, PICTURE_HEIGHT, PICTURE_WIDTH];
                Planned {
                    step,
                    shape: shapes[Picture as usize],
                    per_unit: PICTURE_HEIGHT,
                    units: 1,
                }
            }
            Step::Conv { conv, from, to, .. } => {
                let conv = &convs[conv];
                let [_, rows, columns] = shapes[from[0] as usize];
                let shape = [
                    conv.outputs(),
                    rows / conv.stride(),
                    columns / conv.stride(),
                ];
                shapes[to as usize] = shape;
                let per_unit = (UNIT_COST / conv.row_cost(shape[2])).clamp(1, shape[1]);
                Planned {
                    step,
                    shape,
                    per_unit,
                    units: conv.blocks() * shape[1].div_ceil(per_unit),
                }
            }
            Step::Double { from } => {
                let [channels, rows, columns] = shapes[from as usize];
                let shape = [channels, 2 * rows, 2 * columns];
                shapes[Doubled as usize] = shape;
                let per_unit = (UNIT_VALUES / (shape[1] * shape[2])).clamp(1, channels);
                Planned {
                    step,
                    shape,
                    per_unit,
                    units: channels.div_ceil(per_unit),
                }
            }
            Step::Heads => {
                let row_cost: usize = heads
                    .iter()
                    .map(|head| {
                        head.conv.blocks() * head.conv.row_cost(MAP_WIDTH)
                            + head.out.outputs() * HEAD_CHANNELS * MAP_WIDTH
                    })
                    .max()
                    .unwrap_or(1);
                let per_unit = (UNIT_COST / row_cost).clamp(1, MAP_HEIGHT);
                Planned {
                    step,
                    shape: [HEAD_CHANNELS, MAP_HEIGHT, MAP_WIDTH],
                    per_unit,
                    units: heads.len() * MAP_HEIGHT.div_ceil(per_unit),
                }
            }
        };
        plan.push(planned);
    }
    plan
}

pub(super) fn total_units(plan: &[Planned]) -> u32 {
    plan.iter().map(|planned| planned.units as u32).sum()
}

/// One reading of the main network, part way through.
///
/// Start one with [`SpineNet::start`], call [`SpineReading::advance`] until
/// [`SpineReading::is_done`], then take the maps with
/// [`SpineReading::into_maps`]. However the units are spread over calls,
/// the maps come out with the same bits.
pub struct SpineReading<'net> {
    net: &'net SpineNet,
    picture: Vec<u8>,
    slots: [Option<Grid>; SLOTS],
    /// The heads' first convolution, a band of rows at a time.
    scratch: Option<Grid>,
    maps: Option<HeadMaps>,
    step: usize,
    unit_in_step: usize,
    units_done: u32,
    just_finished: Option<(&'static str, Slot)>,
}

impl<'net> SpineReading<'net> {
    pub(super) fn new(
        net: &'net SpineNet,
        picture: &[u8],
    ) -> Result<SpineReading<'net>, EngineError> {
        if picture.len() != PICTURE_WIDTH * PICTURE_HEIGHT {
            return Err(EngineError::new(
                ErrorCode::EngineOutOfSync,
                "the trained finder reads a picture of exactly 384 by 768",
            )
            .with_detail(format!("{} values", picture.len())));
        }
        let mut kept = Vec::new();
        kept.try_reserve_exact(picture.len())
            .map_err(|_| out_of_memory())?;
        kept.extend_from_slice(picture);
        Ok(SpineReading {
            net,
            picture: kept,
            slots: Default::default(),
            scratch: None,
            maps: None,
            step: 0,
            unit_in_step: 0,
            units_done: 0,
            just_finished: None,
        })
    }

    /// How many units the whole reading takes.
    pub fn units(&self) -> u32 {
        total_units(&self.net.plan)
    }

    /// How many units are done.
    pub fn units_done(&self) -> u32 {
        self.units_done
    }

    pub fn is_done(&self) -> bool {
        self.step == self.net.plan.len()
    }

    /// Does at most `units` units of work; nothing once the reading is
    /// done. Fails only when memory runs out, and then the unit that failed
    /// is the next one tried.
    pub fn advance(&mut self, units: u32) -> Result<(), EngineError> {
        for _ in 0..units {
            if self.is_done() {
                break;
            }
            self.just_finished = None;
            self.run_unit()?;
            self.units_done += 1;
        }
        Ok(())
    }

    /// The block the last unit completed, with its map, if the last unit
    /// completed one: `"e1"` to `"e5"`, `"d4"`, `"d3"` or `"d2"`. Only the
    /// checks against the reference look at these.
    pub fn just_finished(&self) -> Option<(&'static str, &Grid)> {
        let (name, slot) = self.just_finished?;
        Some((name, self.slots[slot as usize].as_ref()?))
    }

    /// The three maps, once the reading is done.
    pub fn maps(&self) -> Option<&HeadMaps> {
        self.maps.as_ref().filter(|_| self.is_done())
    }

    /// The three maps, once the reading is done.
    pub fn into_maps(self) -> Option<HeadMaps> {
        if self.is_done() { self.maps } else { None }
    }

    fn run_unit(&mut self) -> Result<(), EngineError> {
        let planned = self.net.plan[self.step];
        let unit = self.unit_in_step;
        match planned.step {
            Step::Picture => self.fill_picture()?,
            Step::Conv { conv, from, to, .. } => {
                let bands = planned.shape[1].div_ceil(planned.per_unit);
                let (block, band) = (unit / bands, unit % bands);
                let start = band * planned.per_unit;
                let rows = start..(start + planned.per_unit).min(planned.shape[1]);
                let mut out = self.take_or_make(to, planned.shape)?;
                let ran = self.held(from).map(|inputs| {
                    self.net.convs[conv].run(&inputs, block, rows, &mut out, start);
                });
                self.slots[to as usize] = Some(out);
                ran?;
            }
            Step::Double { from } => {
                let start = unit * planned.per_unit;
                let channels = start..(start + planned.per_unit).min(planned.shape[0]);
                let mut out = self.take_or_make(Doubled, planned.shape)?;
                if let Some(source) = &self.slots[from as usize] {
                    upsample_twice(source, channels, &mut out);
                }
                self.slots[Doubled as usize] = Some(out);
                self.held(&[from])?;
            }
            Step::Heads => self.run_heads(planned, unit)?,
        }
        self.unit_in_step += 1;
        if self.unit_in_step == planned.units {
            self.finish_step(planned.step);
        }
        Ok(())
    }

    fn finish_step(&mut self, step: Step) {
        match step {
            Step::Picture => self.picture = Vec::new(),
            Step::Conv {
                to, drop, finishes, ..
            } => {
                for &slot in drop {
                    self.slots[slot as usize] = None;
                }
                self.just_finished = finishes.map(|name| (name, to));
            }
            Step::Double { from } => self.slots[from as usize] = None,
            Step::Heads => {
                self.slots[Decoded as usize] = None;
                self.scratch = None;
            }
        }
        self.step += 1;
        self.unit_in_step = 0;
    }

    /// The input is each byte over 255, less a half, in single precision:
    /// a division, not a multiplication by 1/255, as the reference does it.
    fn fill_picture(&mut self) -> Result<(), EngineError> {
        let mut grid = Grid::bordered(1, PICTURE_HEIGHT, PICTURE_WIDTH)?;
        for (row, bytes) in self
            .picture
            .as_chunks::<PICTURE_WIDTH>()
            .0
            .iter()
            .enumerate()
        {
            for (value, &byte) in grid.row_mut(0, row).iter_mut().zip(bytes) {
                *value = f32::from(byte) / 255.0 - 0.5;
            }
        }
        self.slots[Picture as usize] = Some(grid);
        Ok(())
    }

    /// Each unit is one head over a band of rows: its 3x3 convolution for
    /// those rows into a scratch map, then its 1x1 convolution from there
    /// into the head's maps, so the heads' 64-channel maps are never held
    /// whole.
    fn run_heads(&mut self, planned: Planned, unit: usize) -> Result<(), EngineError> {
        if self.maps.is_none() {
            self.maps = Some(HeadMaps {
                heat: Grid::zeros(HEADS[0].1, MAP_HEIGHT, MAP_WIDTH)?,
                corners: Grid::zeros(HEADS[1].1, MAP_HEIGHT, MAP_WIDTH)?,
                centre_in_cell: Grid::zeros(HEADS[2].1, MAP_HEIGHT, MAP_WIDTH)?,
            });
        }
        if self.scratch.is_none() {
            self.scratch = Some(Grid::zeros(HEAD_CHANNELS, planned.per_unit, MAP_WIDTH)?);
        }
        let bands = MAP_HEIGHT.div_ceil(planned.per_unit);
        let (head_index, band) = (unit / bands, unit % bands);
        let start = band * planned.per_unit;
        let end = (start + planned.per_unit).min(MAP_HEIGHT);
        let decoded = self.slots[Decoded as usize].as_ref().ok_or_else(lost)?;
        let (Some(scratch), Some(maps)) = (self.scratch.as_mut(), self.maps.as_mut()) else {
            return Err(lost());
        };
        let head = &self.net.heads[head_index];
        for block in 0..head.conv.blocks() {
            head.conv.run(&[decoded], block, start..end, scratch, 0);
        }
        let out = match head_index {
            0 => &mut maps.heat,
            1 => &mut maps.corners,
            _ => &mut maps.centre_in_cell,
        };
        head.out.run(scratch, 0..end - start, out, start);
        Ok(())
    }

    /// The map in `slot`, or a new bordered one of `shape` if this is the
    /// step's first unit.
    fn take_or_make(&mut self, slot: Slot, shape: [usize; 3]) -> Result<Grid, EngineError> {
        match self.slots[slot as usize].take() {
            Some(grid) => Ok(grid),
            None => Grid::bordered(shape[0], shape[1], shape[2]),
        }
    }

    fn held(&self, slots: &[Slot]) -> Result<Vec<&Grid>, EngineError> {
        slots
            .iter()
            .map(|&slot| self.slots[slot as usize].as_ref().ok_or_else(lost))
            .collect()
    }
}

fn out_of_memory() -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfMemory,
        "not enough memory for the trained finder's maps",
    )
}

/// A map a step needs is not held: the steps are out of order, which only
/// a mistake in this file could cause.
fn lost() -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfSync,
        "a map the trained finder needs was let go too early",
    )
}

#[cfg(test)]
mod tests {
    use super::super::BLOCKS;
    use super::*;

    #[test]
    fn every_slot_a_step_reads_was_written_by_an_earlier_step_and_not_yet_let_go() {
        let mut held = [false; SLOTS];
        for step in STEPS {
            match step {
                Step::Picture => held[Picture as usize] = true,
                Step::Conv { from, to, drop, .. } => {
                    assert!(from.iter().all(|&slot| held[slot as usize]), "{step:?}");
                    assert!(!held[to as usize], "a map is written afresh: {step:?}");
                    held[to as usize] = true;
                    for &slot in drop {
                        held[slot as usize] = false;
                    }
                }
                Step::Double { from } => {
                    assert!(held[from as usize], "{step:?}");
                    assert!(!held[Doubled as usize], "a map is written afresh: {step:?}");
                    held[Doubled as usize] = true;
                    held[from as usize] = false;
                }
                Step::Heads => {
                    assert!(held[Decoded as usize]);
                    held[Decoded as usize] = false;
                }
            }
        }
        assert_eq!(held, [false; SLOTS], "everything is let go by the end");
    }

    fn net() -> &'static SpineNet {
        crate::weights::Weights::baked()
            .expect("the compiled-in weights read")
            .spine_net()
    }

    #[test]
    fn a_picture_of_the_wrong_size_is_refused() {
        let refused = net().start(&[0; 384 * 767]).err().expect("too short");
        assert_eq!(refused.code, ErrorCode::EngineOutOfSync);
        assert!(net().start(&[0; 384 * 769]).is_err());
    }

    #[test]
    fn no_unit_runs_on_zero_and_units_are_counted() {
        let net = net();
        let mut reading = net
            .start(&[128; PICTURE_WIDTH * PICTURE_HEIGHT])
            .expect("the right size");
        assert_eq!(reading.units(), net.units());
        reading.advance(0).expect("nothing to do");
        assert_eq!(reading.units_done(), 0);
        assert!(reading.maps().is_none());
        reading.advance(3).expect("memory enough");
        assert_eq!(reading.units_done(), 3);
        assert!(!reading.is_done());
    }

    #[test]
    fn every_unit_stays_within_its_share_of_the_work() {
        let net = net();
        let mut total = 0;
        for planned in &net.plan {
            if let Step::Conv { conv, .. } = planned.step {
                let row_cost = net.convs[conv].row_cost(planned.shape[2]);
                assert!(
                    planned.per_unit * row_cost <= UNIT_COST || planned.per_unit == 1,
                    "{planned:?}"
                );
            }
            if let Step::Double { .. } = planned.step {
                assert!(
                    planned.per_unit * planned.shape[1] * planned.shape[2] <= UNIT_VALUES,
                    "{planned:?}"
                );
            }
            total += planned.units;
        }
        assert_eq!(total as u32, net.units());
    }

    #[test]
    fn the_steps_name_every_block_once_in_order() {
        let names: Vec<&str> = STEPS
            .iter()
            .filter_map(|step| match step {
                Step::Conv { finishes, .. } => *finishes,
                _ => None,
            })
            .collect();
        let blocks: Vec<&str> = BLOCKS.iter().map(|block| block.0).collect();
        assert_eq!(names, blocks);
    }
}
