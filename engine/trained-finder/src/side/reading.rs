//! A separate fixed schedule using the existing convolution sum order.

use super::{lost, network::SideNetwork};
use crate::{grid::Grid, layers::resample::upsample_twice};
use shapes::errors::EngineError;
use std::rc::Rc;

const COST: usize = 16 << 20;
const VALUES: usize = 1 << 19;
const PICTURE: usize = 0;
const MIDDLE: usize = 1;
const DOUBLED: usize = 7;
const DECODED: usize = 8;

#[derive(Clone, Copy, Debug)]
enum Step {
    Picture,
    Conv {
        index: usize,
        from: &'static [usize],
        to: usize,
        drop: &'static [usize],
    },
    Double {
        from: usize,
    },
    Head,
}

const fn conv(index: usize, from: &'static [usize], to: usize, drop: &'static [usize]) -> Step {
    Step::Conv {
        index,
        from,
        to,
        drop,
    }
}
const STEPS: [Step; 21] = [
    Step::Picture,
    conv(0, &[PICTURE], MIDDLE, &[PICTURE]),
    conv(1, &[MIDDLE], 2, &[MIDDLE]),
    conv(2, &[2], MIDDLE, &[2]),
    conv(3, &[MIDDLE], 3, &[MIDDLE]),
    conv(4, &[3], MIDDLE, &[]),
    conv(5, &[MIDDLE], 4, &[MIDDLE]),
    conv(6, &[4], MIDDLE, &[]),
    conv(7, &[MIDDLE], 5, &[MIDDLE]),
    conv(8, &[5], MIDDLE, &[]),
    conv(9, &[MIDDLE], 6, &[MIDDLE]),
    Step::Double { from: 6 },
    conv(10, &[DOUBLED, 5], MIDDLE, &[DOUBLED, 5]),
    conv(11, &[MIDDLE], DECODED, &[MIDDLE]),
    Step::Double { from: DECODED },
    conv(12, &[DOUBLED, 4], MIDDLE, &[DOUBLED, 4]),
    conv(13, &[MIDDLE], DECODED, &[MIDDLE]),
    Step::Double { from: DECODED },
    conv(14, &[DOUBLED, 3], MIDDLE, &[DOUBLED, 3]),
    conv(15, &[MIDDLE], DECODED, &[MIDDLE]),
    Step::Head,
];

#[derive(Clone, Copy, Debug)]
struct Planned {
    step: Step,
    shape: [usize; 3],
    per_unit: usize,
    units: usize,
}

fn plan(net: &SideNetwork, height: usize, width: usize) -> Vec<Planned> {
    let mut shapes = [[0; 3]; 9];
    STEPS
        .iter()
        .map(|&step| match step {
            Step::Picture => {
                shapes[PICTURE] = [1, height, width];
                Planned {
                    step,
                    shape: shapes[PICTURE],
                    per_unit: 65_536,
                    units: (height * width).div_ceil(65_536),
                }
            }
            Step::Conv {
                index, from, to, ..
            } => {
                let c = &net.convs[index];
                let [_, h, w] = shapes[from[0]];
                let shape = [c.outputs(), h / c.stride(), w / c.stride()];
                shapes[to] = shape;
                let per_unit = (COST / c.row_cost(shape[2])).clamp(1, shape[1]);
                Planned {
                    step,
                    shape,
                    per_unit,
                    units: c.blocks() * shape[1].div_ceil(per_unit),
                }
            }
            Step::Double { from } => {
                let [c, h, w] = shapes[from];
                let shape = [c, h * 2, w * 2];
                shapes[DOUBLED] = shape;
                let per_unit = (VALUES / (shape[1] * shape[2])).clamp(1, c);
                Planned {
                    step,
                    shape,
                    per_unit,
                    units: c.div_ceil(per_unit),
                }
            }
            Step::Head => {
                let [_, h, w] = shapes[DECODED];
                let c = &net.convs[16];
                let cost = c.blocks() * c.row_cost(w) + net.head.outputs() * 64 * w;
                let per_unit = (COST / cost).clamp(1, h);
                Planned {
                    step,
                    shape: [20, h, w],
                    per_unit,
                    units: h.div_ceil(per_unit),
                }
            }
        })
        .collect()
}

/// A reading owns its input and shares its packed network without a clone.
pub struct SideReading {
    net: Rc<SideNetwork>,
    picture: Vec<u8>,
    slots: [Option<Grid>; 9],
    plan: Vec<Planned>,
    step: usize,
    unit: usize,
    done: u32,
    maps: Option<Grid>,
    scratch: Option<Grid>,
    last: Option<(usize, usize)>,
    #[cfg(test)]
    watch_head: Vec<usize>,
    #[cfg(test)]
    head_samples: Vec<(usize, f32)>,
}

impl SideReading {
    pub(crate) fn new(
        net: Rc<SideNetwork>,
        picture: Vec<u8>,
        height: usize,
        width: usize,
    ) -> Result<Self, EngineError> {
        if picture.len() != width * height
            || width == 0
            || height == 0
            || !width.is_multiple_of(32)
            || !height.is_multiple_of(32)
        {
            return Err(lost());
        }
        let plan = plan(&net, height, width);
        Ok(Self {
            net,
            picture,
            slots: Default::default(),
            plan,
            step: 0,
            unit: 0,
            done: 0,
            maps: None,
            scratch: None,
            last: None,
            #[cfg(test)]
            watch_head: Vec::new(),
            #[cfg(test)]
            head_samples: Vec::new(),
        })
    }
    pub fn units(&self) -> u32 {
        self.plan.iter().map(|p| p.units as u32).sum()
    }
    pub fn units_done(&self) -> u32 {
        self.done
    }
    pub fn is_done(&self) -> bool {
        self.step == self.plan.len()
    }
    pub fn maps(&self) -> Option<&Grid> {
        self.maps.as_ref().filter(|_| self.is_done())
    }
    pub fn into_maps(self) -> Result<Grid, EngineError> {
        if self.is_done() {
            self.maps.ok_or_else(lost)
        } else {
            Err(lost())
        }
    }

    /// The last completed convolution, before its map is consumed.
    pub fn just_finished(&self) -> Option<(usize, &Grid)> {
        let (index, slot) = self.last?;
        Some((index, self.slots[slot].as_ref()?))
    }

    pub fn advance(&mut self, units: u32) -> Result<(), EngineError> {
        for _ in 0..units {
            if self.is_done() {
                break;
            }
            self.last = None;
            self.run_unit()?;
            self.done += 1;
        }
        Ok(())
    }

    fn run_unit(&mut self) -> Result<(), EngineError> {
        let p = self.plan[self.step];
        match p.step {
            Step::Picture => {
                let mut grid = self.take(PICTURE, p.shape)?;
                let start = self.unit * p.per_unit;
                let end = (start + p.per_unit).min(self.picture.len());
                for i in start..end {
                    grid.row_mut(0, i / p.shape[2])[i % p.shape[2]] =
                        f32::from(self.picture[i]) / 255.0 - 0.5;
                }
                self.slots[PICTURE] = Some(grid);
            }
            Step::Conv {
                index, from, to, ..
            } => {
                let bands = p.shape[1].div_ceil(p.per_unit);
                let block = self.unit / bands;
                let start = self.unit % bands * p.per_unit;
                let end = (start + p.per_unit).min(p.shape[1]);
                let mut out = self.take(to, p.shape)?;
                let first = self.slots[from[0]].as_ref().ok_or_else(lost)?;
                if from.len() == 1 {
                    self.net.convs[index].run(&[first], block, start..end, &mut out, start);
                } else {
                    let second = self.slots[from[1]].as_ref().ok_or_else(lost)?;
                    self.net.convs[index].run(&[first, second], block, start..end, &mut out, start);
                }
                self.slots[to] = Some(out);
            }
            Step::Double { from } => {
                let mut out = self.take(DOUBLED, p.shape)?;
                let start = self.unit * p.per_unit;
                upsample_twice(
                    self.slots[from].as_ref().ok_or_else(lost)?,
                    start..(start + p.per_unit).min(p.shape[0]),
                    &mut out,
                );
                self.slots[DOUBLED] = Some(out);
            }
            Step::Head => self.head(p)?,
        }
        self.unit += 1;
        if self.unit == p.units {
            match p.step {
                Step::Picture => self.picture = Vec::new(),
                Step::Conv {
                    index, to, drop, ..
                } => {
                    for &slot in drop {
                        self.slots[slot] = None;
                    }
                    self.last = Some((index, to));
                }
                Step::Double { from } => self.slots[from] = None,
                Step::Head => {
                    self.slots[DECODED] = None;
                    self.scratch = None;
                }
            }
            self.step += 1;
            self.unit = 0;
        }
        Ok(())
    }

    fn head(&mut self, p: Planned) -> Result<(), EngineError> {
        if self.maps.is_none() {
            self.maps = Some(Grid::zeros(20, p.shape[1], p.shape[2])?);
        }
        if self.scratch.is_none() {
            self.scratch = Some(Grid::zeros(64, p.per_unit, p.shape[2])?);
        }
        let start = self.unit * p.per_unit;
        let end = (start + p.per_unit).min(p.shape[1]);
        let input = self.slots[DECODED].as_ref().ok_or_else(lost)?;
        let scratch = self.scratch.as_mut().ok_or_else(lost)?;
        for block in 0..self.net.convs[16].blocks() {
            self.net.convs[16].run(&[input], block, start..end, scratch, 0);
        }
        #[cfg(test)]
        for &index in &self.watch_head {
            let plane = p.shape[1] * p.shape[2];
            let c = index / plane;
            let y = index % plane / p.shape[2];
            let x = index % p.shape[2];
            if y >= start && y < end {
                self.head_samples.push((index, scratch.at(c, y - start, x)));
            }
        }
        self.net.head.run(
            scratch,
            0..end - start,
            self.maps.as_mut().ok_or_else(lost)?,
            start,
        );
        Ok(())
    }

    fn take(&mut self, slot: usize, [c, h, w]: [usize; 3]) -> Result<Grid, EngineError> {
        match self.slots[slot].take() {
            Some(grid) => Ok(grid),
            None => Grid::bordered(c, h, w),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::side::{decode, table, weights::WeightSetup};
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};

    fn hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn next(state: &mut u32) -> u32 {
        *state ^= *state << 13;
        *state ^= *state >> 17;
        *state ^= *state << 5;
        *state
    }

    fn synthetic_packet() -> Vec<u8> {
        let mut payload = Vec::new();
        let mut tensors = Vec::new();
        let mut state = 17;
        for layer in table::layers() {
            for (suffix, shape) in [
                (
                    "weight",
                    vec![layer.outputs, layer.inputs, layer.kernel, layer.kernel],
                ),
                ("bias", vec![layer.outputs]),
            ] {
                let count = shape.iter().product::<usize>();
                tensors.push(json!({"name":format!("{}.{}",layer.name,suffix),"shape":shape,"offset":payload.len(),"count":count}));
                for _ in 0..count {
                    let value = if suffix == "bias" {
                        0.0
                    } else {
                        ((next(&mut state) >> 24) as i32 - 128) as f32 / 16384.0
                    };
                    payload.extend(value.to_le_bytes());
                }
            }
        }
        let header=serde_json::to_vec(&json!({"format":"side-finder-weights","version":1,"model":"side_lumbar_heatmap_v1","checkpointSha256":table::CHECKPOINT,"batchNormEps":0.00001,"batchNormFolded":true,"inputShape":[1,512,384],"outputShape":[20,128,96],"bytes":payload.len(),"payloadSha256":hex(&payload),"tensors":tensors})).unwrap();
        let mut bytes = b"SWSIDE01".to_vec();
        bytes.extend((header.len() as u32).to_le_bytes());
        bytes.extend(header);
        bytes.extend(payload);
        bytes
    }

    fn compare(grid: &Grid, expected: &Value, largest: &mut f64, ratio: &mut f64) {
        assert_eq!(
            vec![grid.channels(), grid.rows(), grid.columns()],
            expected["shape"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| n.as_u64().unwrap() as usize)
                .collect::<Vec<_>>()
        );
        for sample in expected["samples"].as_array().unwrap() {
            let i = sample["index"].as_u64().unwrap() as usize;
            let plane = grid.rows() * grid.columns();
            check(
                grid.at(i / plane, i % plane / grid.columns(), i % grid.columns()),
                sample["value"].as_f64().unwrap(),
                largest,
                ratio,
            );
        }
    }

    fn check(actual: f32, expected: f64, largest: &mut f64, ratio: &mut f64) {
        let gap = (f64::from(actual) - expected).abs();
        let tolerance = 2e-4 + 1e-4 * expected.abs();
        *largest = largest.max(gap);
        *ratio = ratio.max(gap / tolerance);
        assert!(
            actual.is_finite() && gap <= tolerance,
            "{actual} versus {expected}: {gap}"
        );
    }

    fn known(bytes: &[u8], text: &str) {
        let expected: Value = serde_json::from_str(text).unwrap();
        let mut setup = WeightSetup::new(bytes);
        assert_eq!(setup.units_done(), 0);
        assert!(setup.finish().is_err());
        setup = WeightSetup::new(bytes);
        while !setup.advance().unwrap() {}
        assert_eq!(setup.units_done(), WeightSetup::units());
        let net = Rc::new(setup.finish().unwrap());
        let h = expected["input"]["shape"][1].as_u64().unwrap() as usize;
        let w = expected["input"]["shape"][2].as_u64().unwrap() as usize;
        let mut state = 20261113;
        let picture: Vec<_> = (0..h * w).map(|_| (next(&mut state) >> 24) as u8).collect();
        assert_eq!(
            hex(&picture),
            expected["input"]["bytesSha256"].as_str().unwrap()
        );
        let mut reading = SideReading::new(net, picture, h, w).unwrap();
        reading.watch_head = expected["layers"]["landmarks.0"]["samples"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["index"].as_u64().unwrap() as usize)
            .collect();
        reading.advance(0).unwrap();
        assert_eq!(reading.units_done(), 0);
        assert!(reading.maps().is_none());
        let mut largest = 0.0f64;
        let mut ratio = 0.0f64;
        let mut count = 0;
        let layers = table::layers();
        for p in &reading.plan {
            match p.step {
                Step::Conv { index, .. } => {
                    assert!(p.per_unit * reading.net.convs[index].row_cost(p.shape[2]) <= COST)
                }
                Step::Double { .. } => assert!(p.per_unit * p.shape[1] * p.shape[2] <= VALUES),
                Step::Head => assert!(
                    p.per_unit
                        * (reading.net.convs[16].blocks()
                            * reading.net.convs[16].row_cost(p.shape[2])
                            + 20 * 64 * p.shape[2])
                        <= COST
                ),
                Step::Picture => assert!(p.per_unit <= 65_536),
            }
        }
        while !reading.is_done() {
            reading.advance(1).unwrap();
            if let Some((index, grid)) = reading.just_finished() {
                compare(
                    grid,
                    &expected["layers"][&layers[index].name],
                    &mut largest,
                    &mut ratio,
                );
                count += 1;
            }
        }
        assert_eq!(count, 16);
        assert_eq!(reading.units_done(), reading.units());
        assert!(reading.slots.iter().all(Option::is_none));
        assert!(reading.scratch.is_none());
        assert!(reading.picture.is_empty());
        for sample in expected["layers"]["landmarks.0"]["samples"]
            .as_array()
            .unwrap()
        {
            let i = sample["index"].as_u64().unwrap() as usize;
            let found: Vec<_> = reading.head_samples.iter().filter(|s| s.0 == i).collect();
            assert_eq!(found.len(), 1);
            check(
                found[0].1,
                sample["value"].as_f64().unwrap(),
                &mut largest,
                &mut ratio,
            );
        }
        let maps = reading.maps().unwrap();
        compare(
            maps,
            &expected["layers"]["landmarks.2"],
            &mut largest,
            &mut ratio,
        );
        let mut point_gap = 0.0f64;
        for channel in 0..20 {
            let point = decode::channel(maps, channel).unwrap();
            for (axis, &actual) in point.iter().enumerate() {
                let reference = expected["points"][channel / 4][channel % 4][axis]
                    .as_f64()
                    .unwrap();
                point_gap = point_gap.max((f64::from(actual) - reference).abs());
            }
        }
        assert!(point_gap <= 0.01, "point gap {point_gap}");
        let hash = hex(&(0..20)
            .flat_map(|c| maps.plane(c).iter().flat_map(|v| v.to_le_bytes()))
            .collect::<Vec<_>>());
        eprintln!(
            "{}: 18 layers, 108 samples, maximum sample error {largest:e}, tolerance share {ratio:e}, maximum point error {point_gap:e}, logits sha256 {hash}, {} network units",
            expected["kind"],
            reading.units()
        );
        let done = reading.units_done();
        reading.advance(u32::MAX).unwrap();
        assert_eq!(reading.units_done(), done);
    }

    #[test]
    fn the_entire_synthetic_network_matches_without_optional_weights() {
        known(
            &synthetic_packet(),
            include_str!("../../tests/side-known-answers/synthetic.json"),
        );
    }

    #[test]
    fn the_present_network_matches_the_artificial_input_answer() {
        if let Some(bytes) = crate::side::weights::packet() {
            known(
                bytes,
                include_str!("../../tests/side-known-answers/trained.json"),
            );
        }
    }
}
