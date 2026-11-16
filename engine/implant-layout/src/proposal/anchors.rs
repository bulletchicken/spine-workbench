//! Apply patterns in a curve-relative frame and refresh retained look sources.

use crate::{
    LayoutError,
    patterns::{self, Pattern},
};
use nalgebra::Vector3;
use shapes::implant_layout::{AnchorKind, AnchorProposal, LookCode, ValueStatus};
use shapes::levels::Level;
use shapes::spine_readings::BodySide;

pub(super) fn apply(
    anchors: &mut [AnchorProposal],
    levels: &[Level],
    apex: Option<Level>,
    convex: Option<BodySide>,
    pattern: Pattern,
) -> Result<(), LayoutError> {
    // This project's canonical right-convex frame makes alternating
    // non-key sides reflect with the curve. Bilateral keys stay paired.
    let reflected = pattern == Pattern::KeyLevels && convex == Some(BodySide::Left);
    if reflected {
        swap(anchors);
    }
    patterns::apply(anchors, levels, apex, pattern)?;
    if reflected {
        swap(anchors);
    }
    for anchor in anchors {
        crate::screws::refresh_look_sources(anchor);
    }
    Ok(())
}

/// This project's fixed sweep budget for `line_up`, independent of clocks.
const LINE_UP_SWEEPS: usize = 200;

/// This project's own inference: an entry is the generic body's front exit
/// less the table chord, so each level inherits its own printed chord mean
/// and fitted body depth apart from its neighbours. One short chord cell
/// (Upendra's T10 convex mean is 10 mm under T9's) or a thin fitted body
/// then puts one seat well off the line of the others, and the rod through
/// every seat kinks there. The chord is printed with a spread, so within
/// one reported SD of its mean each screw's entry slides along its own path
/// to where the rod seats behind the entries line up most closely: the sum,
/// over inner seats, of the squared change in seat-to-seat direction per mm
/// is made as small as those spreads allow. A chord one SD under its mean
/// still holds the length the 90% rule chose, since that rule already held
/// against nine in ten sampled chords. A screw held at the catalogue floor
/// (`shorter-than-tables`) failed that rule, so it slides back only, never
/// towards the front of the body. Directions, sizes and kinds keep their
/// values; hooks, and screws with no printed chord spread, stay put. It
/// runs once over a whole fitted column before any span is cut from it or
/// any pattern applied, so every span, pattern and hand rebuild shares the
/// same entry at each level.
pub(super) fn line_up(anchors: &mut [AnchorProposal]) -> Result<(), LayoutError> {
    let (offset, _) = crate::rods::head_offset()?;
    for side in [BodySide::Left, BodySide::Right] {
        let rows: Vec<usize> = (0..anchors.len())
            .filter(|&i| {
                let a = &anchors[i];
                a.side == side
                    && a.kind != AnchorKind::None
                    && a.entry_status != ValueStatus::Missing
                    && a.direction_status != ValueStatus::Missing
                    && a.entry_mm
                        .iter()
                        .chain(&a.direction_unit)
                        .all(|v| v.is_finite())
            })
            .collect();
        let n = rows.len();
        if n < 3 {
            continue;
        }
        let direction: Vec<Vector3<f64>> = rows
            .iter()
            .map(|&i| Vector3::from(anchors[i].direction_unit))
            .collect();
        let seat: Vec<Vector3<f64>> = rows
            .iter()
            .zip(&direction)
            .map(|(&i, d)| Vector3::from(anchors[i].entry_mm) - d * offset)
            .collect();
        // How far each entry may slide back (out of the bone) and forward.
        let limit: Vec<(f64, f64)> = rows
            .iter()
            .map(|&i| {
                let a = &anchors[i];
                let sd = match (a.kind, a.chord_mm) {
                    (AnchorKind::Screw, Some(mean)) => {
                        crate::screws::anatomy::chord_spread(a.level, a.pedicle.curve_side, mean)
                            .filter(|sd| sd.is_finite() && *sd > 0.0)
                            .unwrap_or(0.0)
                    }
                    _ => 0.0,
                };
                let floored = a.looks.contains(&LookCode::ShorterThanTables);
                (sd, if floored { 0.0 } else { sd })
            })
            .collect();
        let step: Vec<f64> = seat.windows(2).map(|p| (p[1] - p[0]).norm()).collect();
        if step.iter().any(|h| !h.is_finite() || *h <= 0.0) {
            continue;
        }
        let mut slide = vec![0.0; n];
        let at = |slide: &[f64], k: usize| seat[k] + direction[k] * slide[k];
        // Change in unit-length direction at inner seat j.
        let bend = |slide: &[f64], j: usize| {
            (at(slide, j + 1) - at(slide, j)) / step[j]
                - (at(slide, j) - at(slide, j - 1)) / step[j - 1]
        };
        for _ in 0..LINE_UP_SWEEPS {
            for i in 0..n {
                if limit[i] == (0.0, 0.0) {
                    continue;
                }
                let (mut numerator, mut denominator) = (0.0, 0.0);
                for j in i.saturating_sub(1)..=(i + 1).min(n - 2) {
                    if j == 0 {
                        continue;
                    }
                    let weight = if j + 1 == i {
                        1.0 / step[j]
                    } else if j == i {
                        -(1.0 / step[j - 1] + 1.0 / step[j])
                    } else {
                        1.0 / step[j - 1]
                    };
                    let rest = bend(&slide, j) - direction[i] * (weight * slide[i]);
                    numerator += weight * rest.dot(&direction[i]);
                    denominator += weight * weight;
                }
                if denominator > 0.0 {
                    slide[i] = (-numerator / denominator).clamp(-limit[i].0, limit[i].1);
                }
            }
        }
        for (k, &i) in rows.iter().enumerate() {
            anchors[i].entry_mm =
                (Vector3::from(anchors[i].entry_mm) + direction[k] * slide[k]).into();
        }
    }
    Ok(())
}

fn swap(anchors: &mut [AnchorProposal]) {
    for pair in anchors.as_chunks_mut::<2>().0 {
        pair.swap(0, 1);
        pair[0].side = BodySide::Left;
        pair[1].side = BodySide::Right;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screws::{refresh_look_sources, sizing};
    use shapes::implant_layout::{
        AnchorKind, CurveSide, DiameterRule, LookCode, PedicleEstimate, ValueStatus,
    };
    use shapes::spine_readings::Band;

    #[test]
    fn a_missing_secondary_size_keeps_its_null_without_adding_a_narrow_look() {
        let grid = sizing::diameter_grid();
        // W=4 mm admits 5 mm at 1.25, but 0.80 allows only 3.2 mm,
        // below the 3.5 mm grid floor. The inner channel is 2.5 mm.
        let chosen = sizing::choose_diameter(&[4.0; 10], &grid, 1.25, 0.1).unwrap();
        let secondary = sizing::choose_diameter(&[4.0; 10], &grid, 0.80, 0.1).unwrap();
        assert_eq!(chosen.mm, Some(5.0));
        assert_eq!(secondary.mm, None);
        let constant = |mm| Band {
            p10: mm,
            p50: mm,
            p90: mm,
        };
        let mut anchor = AnchorProposal {
            level: Level::T8,
            side: BodySide::Left,
            kind: AnchorKind::Screw,
            entry_mm: [0.0; 3],
            entry_status: ValueStatus::FromTemplate,
            direction_unit: [1.0, 0.0, 0.0],
            direction_status: ValueStatus::FromTables,
            medial_angle: None,
            medial_angles_beside: Vec::new(),
            convergence_deg: constant(0.0),
            diameter_mm: chosen.mm,
            diameter_also_seen: chosen.also_seen,
            diameter_status: ValueStatus::FromTables,
            width_ratio: Some(1.25),
            diameters_beside: vec![DiameterRule {
                width_ratio: 0.80,
                diameter_mm: secondary.mm,
                source: "misenhimer-1989".into(),
            }],
            screw_to_pedicle: Some(constant(1.25)),
            chord_mm: Some(40.0),
            chord_share: Some(0.8),
            length_mm: Some(30.0),
            length_also_seen: Vec::new(),
            length_status: ValueStatus::FromTables,
            usual_length_mm: Some([30.0, 35.0]),
            pedicle: PedicleEstimate {
                outer_width_mm: Some(constant(4.0)),
                inner_width_mm: Some(constant(2.5)),
                versions_too_narrow: 0,
                versions_inner_under_1_mm: 0,
                curve_side: CurveSide::Concave,
                status: ValueStatus::FromTables,
                sources: vec!["gao-2017".into()],
            },
            wider_than_pedicle: Vec::new(),
            screw_here: None,
            looks: vec![LookCode::ConcaveApexNeedsCt],
            diameter_sources: vec!["sato-2022".into()],
            length_sources: vec!["upendra-2010".into()],
            direction_sources: vec!["upendra-2010".into()],
            look_sources: Vec::new(),
            sources: vec!["misenhimer-1989".into()],
        };
        refresh_look_sources(&mut anchor);
        let mut anchors = vec![anchor.clone(), anchor];
        anchors[1].side = BodySide::Right;
        let original = anchors.clone();
        apply(
            &mut anchors,
            &[Level::T8],
            Some(Level::T8),
            Some(BodySide::Right),
            Pattern::AllPedicles,
        )
        .unwrap();
        assert_eq!(anchors, original);
        let value = serde_json::to_value(&anchors[0]).unwrap();
        assert_eq!(value["looks"], serde_json::json!(["concave-apex-needs-ct"]));
        assert_eq!(
            value["diametersBeside"],
            serde_json::json!([
                {"widthRatio": 0.80, "diameterMm": null, "source": "misenhimer-1989"}
            ])
        );
        assert_eq!(anchors[0].look_sources.len(), 1);
    }

    /// Straight-forward screws on a straight column, each entry at the body
    /// exit less its Upendra convex chord: T10's short printed mean puts its
    /// entry, and its seat, about 10 mm in front of T9's and T11's.
    fn column() -> Vec<AnchorProposal> {
        let mut result = Vec::new();
        for (row, level) in [Level::T8, Level::T9, Level::T10, Level::T11, Level::T12]
            .into_iter()
            .enumerate()
        {
            let chord = crate::screws::anatomy::chord(level, CurveSide::Convex, None).unwrap();
            for side in [BodySide::Left, BodySide::Right] {
                let y = if side == BodySide::Left { 12.0 } else { -12.0 };
                result.push(AnchorProposal {
                    level,
                    side,
                    kind: AnchorKind::Screw,
                    entry_mm: [16.0 - chord.mean, y, 200.0 - 25.0 * row as f64],
                    entry_status: ValueStatus::FromTemplate,
                    direction_unit: [1.0, 0.0, 0.0],
                    direction_status: ValueStatus::FromTables,
                    medial_angle: None,
                    medial_angles_beside: Vec::new(),
                    convergence_deg: Band {
                        p10: 0.0,
                        p50: 0.0,
                        p90: 0.0,
                    },
                    diameter_mm: Some(5.0),
                    diameter_also_seen: Vec::new(),
                    diameter_status: ValueStatus::FromTables,
                    width_ratio: None,
                    diameters_beside: Vec::new(),
                    screw_to_pedicle: None,
                    chord_mm: Some(chord.mean),
                    chord_share: Some(0.8),
                    length_mm: Some(30.0),
                    length_also_seen: Vec::new(),
                    length_status: ValueStatus::FromTables,
                    usual_length_mm: None,
                    pedicle: PedicleEstimate {
                        outer_width_mm: None,
                        inner_width_mm: None,
                        versions_too_narrow: 0,
                        versions_inner_under_1_mm: 0,
                        curve_side: CurveSide::Convex,
                        status: ValueStatus::FromTables,
                        sources: Vec::new(),
                    },
                    wider_than_pedicle: Vec::new(),
                    screw_here: None,
                    looks: Vec::new(),
                    diameter_sources: Vec::new(),
                    length_sources: Vec::new(),
                    direction_sources: Vec::new(),
                    look_sources: Vec::new(),
                    sources: Vec::new(),
                });
            }
        }
        result
    }

    /// Largest front-back step out of line at an inner seat of one side.
    fn worst_step(anchors: &[AnchorProposal], side: BodySide) -> f64 {
        let x: Vec<f64> = anchors
            .iter()
            .filter(|a| a.side == side)
            .map(|a| a.entry_mm[0])
            .collect();
        x.windows(3)
            .map(|w| (w[0] - 2.0 * w[1] + w[2]).abs())
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_short_chord_cell_slides_its_entry_within_its_spread_to_line_up_the_seats() {
        let original = column();
        let mut anchors = original.clone();
        line_up(&mut anchors).unwrap();
        for side in [BodySide::Left, BodySide::Right] {
            let before = worst_step(&original, side);
            let after = worst_step(&anchors, side);
            assert!(before > 14.0, "the T10 cell steps {before} mm out of line");
            assert!(
                after < before / 2.0,
                "{before} mm out of line became {after}"
            );
        }
        for (moved, kept) in anchors.iter().zip(&original) {
            let spread = crate::screws::anatomy::chord_spread(
                kept.level,
                CurveSide::Convex,
                kept.chord_mm.unwrap(),
            )
            .unwrap();
            let slide = moved.entry_mm[0] - kept.entry_mm[0];
            assert!(
                slide.abs() <= spread + 1e-12,
                "{:?} slides {slide}",
                kept.level
            );
            assert_eq!(moved.entry_mm[1..], kept.entry_mm[1..]);
            assert_eq!(moved.direction_unit, kept.direction_unit);
            assert_eq!(moved.length_mm, kept.length_mm);
            assert_eq!(moved.kind, kept.kind);
        }
        // Mirrored sides move alike, and hooks keep their entries.
        for pair in anchors.chunks(2) {
            assert_eq!(pair[0].entry_mm[0], pair[1].entry_mm[0]);
        }
        let mut hooked = original.clone();
        for anchor in &mut hooked {
            anchor.kind = AnchorKind::Hook;
        }
        let kept = hooked.clone();
        line_up(&mut hooked).unwrap();
        assert_eq!(hooked, kept);
    }

    /// T10's short cell slides back and pulls T9 and T11 forward. Held at
    /// the catalogue floor, T9 and T11 failed the 90% rule, so moving their
    /// tips towards the front of the body has nothing behind it.
    #[test]
    fn a_screw_held_at_the_catalogue_floor_never_slides_forward() {
        let free = {
            let mut anchors = column();
            line_up(&mut anchors).unwrap();
            anchors
        };
        let original = column();
        let floored = |a: &AnchorProposal| matches!(a.level, Level::T9 | Level::T11);
        let mut anchors = original.clone();
        for anchor in anchors.iter_mut().filter(|a| floored(a)) {
            anchor.looks.push(LookCode::ShorterThanTables);
        }
        line_up(&mut anchors).unwrap();
        for ((moved, kept), unheld) in anchors.iter().zip(&original).zip(&free) {
            let slide = moved.entry_mm[0] - kept.entry_mm[0];
            if floored(kept) {
                assert!(unheld.entry_mm[0] - kept.entry_mm[0] > 0.5);
                assert!(slide <= 0.0, "{:?} slides forward {slide}", kept.level);
            } else if kept.level == Level::T10 {
                assert!(slide < -1.0, "T10 slides {slide}");
            }
        }
    }
}
