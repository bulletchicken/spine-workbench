//! Splits search a bounded pair of endplates first, widening jointly to
//! neighbouring bodies only when no local pair fits. All weights are
//! experimental. Cached phase/gradient edges provide the endplate cost;
//! without them a polarity-independent local intensity profile is used.

use super::geometry::*;
use super::{refused, window};
use auto_pass::handoffs::FilmPassCache;
use darkroom::sampling;
use film_reader::decoded_film::DecodedFilm;
use shapes::errors::EngineError;
use shapes::marks::{MarkSource, PedicleMark};
use shapes::pixels::Px;
use shapes::vertebra::{FilmMarks, VertebraMarks};

#[derive(Clone, Copy)]
pub(super) struct Sizes {
    pub height: f64,
    pub gap: f64,
}

pub(super) fn reference(marks: &FilmMarks, index: usize) -> Sizes {
    let q = quad(&marks.vertebrae[index].marks);
    let mut heights = Vec::new();
    let mut gaps = Vec::new();
    let lo = index.saturating_sub(2);
    let hi = (index + 3).min(marks.vertebrae.len());
    let axis = unit(sub(midpoint(q[3], q[2]), midpoint(q[0], q[1]))).unwrap_or(Px::new(0.0, 1.0));
    for i in lo..hi {
        if i != index {
            heights.push(height(quad(&marks.vertebrae[i].marks)));
        }
        if i + 1 < hi && i != index && i + 1 != index {
            let a = quad(&marks.vertebrae[i].marks);
            let b = quad(&marks.vertebrae[i + 1].marks);
            let d = dot(sub(midpoint(b[0], b[1]), midpoint(a[3], a[2])), axis);
            if d > 0.0 {
                gaps.push(d);
            }
        }
    }
    let median = |v: &mut Vec<f64>| {
        v.sort_by(f64::total_cmp);
        let n = v.len();
        if n.is_multiple_of(2) {
            0.5 * (v[n / 2 - 1] + v[n / 2])
        } else {
            v[n / 2]
        }
    };
    let h = if heights.is_empty() {
        length(sub(midpoint(q[3], q[2]), midpoint(q[0], q[1]))) / 2.2
    } else {
        median(&mut heights)
    };
    let d = if gaps.is_empty() {
        0.25 * h
    } else {
        median(&mut gaps)
    };
    Sizes {
        height: h,
        gap: d.max(0.05 * h),
    }
}

pub(super) fn sample_cost(
    film: &DecodedFilm,
    cache: Option<&FilmPassCache>,
    points: &[(Px, Px, f64)],
    step: f64,
) -> Vec<f64> {
    let profile: Vec<_> = points
        .iter()
        .map(|&(centre, normal, width)| {
            let n = (0.6 * width / step.max(1.0)).ceil().max(2.0) as usize;
            let mut sum = 0.0;
            let mut count = 0;
            for k in 0..=n {
                let p = add(
                    centre,
                    scale(normal, 0.6 * width * (k as f64 / n as f64 - 0.5)),
                );
                if let Some(v) = sampling::bilinear(&film.image, p.x(), p.y()) {
                    sum += f64::from(v);
                    count += 1;
                }
            }
            if count > 0 { sum / count as f64 } else { 0.0 }
        })
        .collect();
    // Gaussian sigma 1, truncated at three samples and renormalised.
    let smooth: Vec<_> = (0..profile.len())
        .map(|i| {
            let mut sum = 0.0;
            let mut weights = 0.0;
            for delta in -3i32..=3 {
                let j = (i as i64 + i64::from(delta)).clamp(0, profile.len() as i64 - 1) as usize;
                let w = libm::exp(-0.5 * f64::from(delta * delta));
                sum += w * profile[j];
                weights += w;
            }
            sum / weights
        })
        .collect();
    let mut strengths: Vec<_> = (0..smooth.len())
        .map(|i| (smooth[(i + 1).min(smooth.len() - 1)] - smooth[i.saturating_sub(1)]).abs())
        .collect();
    if let Some(cache) = cache {
        let cached: Vec<_> = points
            .iter()
            .map(|&(centre, normal, width)| {
                let n = (0.6 * width / step.max(1.0)).ceil().max(2.0) as usize;
                let mut sum = 0.0;
                let mut count = 0;
                for k in 0..=n {
                    let p = add(
                        centre,
                        scale(normal, 0.6 * width * (k as f64 / n as f64 - 0.5)),
                    );
                    let evidence = &cache.evidence;
                    if let Some((x, y)) = evidence.valid.image_point(p)
                        && sampling::bilinear(&evidence.valid, x, y).is_some_and(|v| v >= 0.999)
                    {
                        let read = |image: &darkroom::grey_image::GreyImage<f32>| {
                            image
                                .image_point(p)
                                .and_then(|(x, y)| sampling::bilinear(image, x, y))
                        };
                        if let (Some(a), Some(b)) =
                            (read(&evidence.phase_edge), read(&evidence.gradient_edge))
                            && a.is_finite()
                            && b.is_finite()
                        {
                            sum += f64::from(a.max(b));
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    Some(sum / count as f64)
                } else {
                    None
                }
            })
            .collect();
        let maximum = cached.iter().flatten().copied().fold(0.0, f64::max);
        if maximum > 1e-9 {
            let fallback = strengths.iter().copied().fold(0.0, f64::max).max(1e-12);
            for (s, c) in strengths.iter_mut().zip(cached) {
                *s = c.map_or(*s / fallback, |v| v / maximum);
            }
        }
    }
    let maximum = strengths.iter().copied().fold(0.0, f64::max);
    strengths
        .iter()
        .map(|v| {
            if maximum > 1e-12 {
                1.0 - v / maximum
            } else {
                1.0
            }
        })
        .collect()
}

pub(super) fn height_cost(h: f64, sizes: Sizes) -> f64 {
    {
        let z = (h - sizes.height) / (0.15 * sizes.height);
        0.5 * z * z
    }
}
pub(super) fn gap_cost(d: f64, sizes: Sizes) -> f64 {
    {
        let z = (d - sizes.gap) / (0.4 * sizes.gap);
        0.5 * z * z
    }
}

pub(super) fn apply(
    film: &DecodedFilm,
    cache: Option<&FilmPassCache>,
    marks: &mut FilmMarks,
    index: usize,
    at: Px,
    new_id: u32,
) -> Result<[Px; 2], EngineError> {
    let q = quad(&marks.vertebrae[index].marks);
    let sizes = reference(marks, index);
    let top = midpoint(q[0], q[1]);
    let axis = sub(midpoint(q[3], q[2]), top);
    let hp = length(axis);
    let u = unit(axis).ok_or_else(|| refused("split-invalid", "The outline has no height"))?;
    let n = Px::new(-u.y(), u.x());
    let step = sizes.height / 50.0;
    if !step.is_finite() || step <= 0.0 {
        return Err(refused(
            "split-invalid",
            "The reference height is not positive",
        ));
    }
    let desired = dot(sub(at, top), u).clamp(0.0, hp);
    let count = (hp / step).ceil() as usize;
    // A grossly overlong parent cannot fit two height ranges; avoid an
    // unbounded image profile and go straight to the bounded window.
    if count <= 200 {
        let points: Vec<_> = (0..=count)
            .map(|i| (add(top, scale(u, i as f64 * step)), n, width(q)))
            .collect();
        let cost = sample_cost(film, cache, &points, step);
        let mut best: Option<(f64, f64, f64)> = None;
        for (a, ca) in cost.iter().enumerate().take(count).skip(1) {
            let e1 = a as f64 * step;
            if !(0.35 * sizes.height..=1.6 * sizes.height).contains(&e1) {
                continue;
            }
            for (b, cb) in cost.iter().enumerate().take(count).skip(a + 1) {
                let e2 = b as f64 * step;
                let d = e2 - e1;
                let h2 = hp - e2;
                if !(0.35 * sizes.height..=1.6 * sizes.height).contains(&h2)
                    || !(0.05 * sizes.height..=0.8 * sizes.height).contains(&d)
                    || (0.5 * (e1 + e2) - desired).abs() > 0.2 * sizes.height
                {
                    continue;
                }
                let energy =
                    ca + cb + height_cost(e1, sizes) + height_cost(h2, sizes) + gap_cost(d, sizes);
                if best.is_none_or(|(old, _, _)| energy < old) {
                    best = Some((energy, e1, e2));
                }
            }
        }
        if let Some((_, e1, e2)) = best {
            return pieces(marks, index, new_id, e1, e2);
        }
    }
    window::split(film, cache, marks, index, at, new_id, sizes)
}

pub(super) fn pieces(
    marks: &mut FilmMarks,
    index: usize,
    new_id: u32,
    e1: f64,
    e2: f64,
) -> Result<[Px; 2], EngineError> {
    let original = &marks.vertebrae[index];
    let old = original.marks.corners();
    let q = old.map(|m| m.at);
    let a = cut(q, e1)
        .ok_or_else(|| refused("split-invalid", "The split line does not cross both sides"))?;
    let b = cut(q, e2)
        .ok_or_else(|| refused("split-invalid", "The split line does not cross both sides"))?;
    let qa = [old[0], old[1], placed(a[1]), placed(a[0])];
    let qb = [placed(b[0]), placed(b[1]), old[2], old[3]];
    if !valid(qa.map(|m| m.at)) || !valid(qb.map(|m| m.at)) {
        return Err(refused(
            "split-invalid",
            "The split would make a folded outline",
        ));
    }
    let mut upper = original.clone();
    let mut lower = original.clone();
    lower.id = new_id;
    put_corners(&mut upper.marks, qa);
    put_corners(&mut lower.marks, qb);
    if let (
        VertebraMarks::Front { points: old },
        VertebraMarks::Front { points: up },
        VertebraMarks::Front { points: down },
    ) = (&original.marks, &mut upper.marks, &mut lower.marks)
    {
        for (pedicle, up_ped, down_ped, u) in [
            (old.ped_l, &mut up.ped_l, &mut down.ped_l, 0.11),
            (old.ped_r, &mut up.ped_r, &mut down.ped_r, 0.89),
        ] {
            let inferred = |quad| PedicleMark::Visible {
                at: bilinear(quad, u, 0.32),
                source: MarkSource::BoundaryWand,
                pinned: false,
                inferred: true,
                ring_score: None,
            };
            *up_ped = inferred(qa.map(|m| m.at));
            *down_ped = inferred(qb.map(|m| m.at));
            if let Some(mark) = pedicle.mark() {
                let distance = |q| {
                    if inside(mark.at, q) {
                        0.0
                    } else {
                        boundary_distance(mark.at, q)
                    }
                };
                if distance(qa.map(|m| m.at)) <= distance(qb.map(|m| m.at)) {
                    *up_ped = pedicle;
                } else {
                    *down_ped = pedicle;
                }
            } else {
                *up_ped = pedicle;
                *down_ped = pedicle;
            }
        }
    }
    upper.outline = None;
    lower.outline = None;
    marks.vertebrae[index] = upper;
    marks.vertebrae.insert(index + 1, lower);
    Ok([midpoint(a[0], b[0]), midpoint(a[1], b[1])])
}
