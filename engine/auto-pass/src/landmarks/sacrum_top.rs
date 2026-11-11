//! The sacral plateau's two ends.

use crate::handoffs::{EndplateOnStrip, Polarity, Strip};
use crate::points::back_to_film::{facing, plateau_ends};
use shapes::film_geometry::ImageRight;
use shapes::landmarks::SacrumTop;
use shapes::marks::Mark;

/// Refine a found plateau and name its ends in the patient's frame.
/// No plateau in the chain, or no usable wall support, leaves it absent.
pub fn place(
    strip: &Strip,
    plateau: Option<EndplateOnStrip>,
    polarity: Polarity,
) -> Option<SacrumTop> {
    let named_for = facing(strip).ok()?;
    let ends = plateau_ends(strip, plateau?, polarity)?;
    Some(named_ends(ends, named_for))
}

fn named_ends([negative, positive]: [Mark; 2], named_for: ImageRight) -> SacrumTop {
    match named_for {
        ImageRight::PatientLeft => SacrumTop::Front {
            sac_l: positive,
            sac_r: negative,
        },
        ImageRight::PatientRight => SacrumTop::Front {
            sac_l: negative,
            sac_r: positive,
        },
        ImageRight::Anterior => SacrumTop::Side {
            sac_a: positive,
            sac_p: negative,
        },
        ImageRight::Posterior => SacrumTop::Side {
            sac_a: negative,
            sac_p: positive,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoffs::{StripEvidence, StripRow};
    use shapes::film::FilmKind;
    use shapes::pixels::Px;

    #[test]
    fn plateau_names_follow_the_patient_even_on_a_turned_film() {
        // Upright right points upward on this stored film. Naming by the
        // film's x coordinates would give no useful ordering at all.
        let negative = Mark::auto(Px::new(12.0, 30.0));
        let positive = Mark::auto(Px::new(12.0, 10.0));
        assert_eq!(
            named_ends([negative, positive], ImageRight::PatientLeft),
            SacrumTop::Front {
                sac_l: positive,
                sac_r: negative
            }
        );
        assert_eq!(
            named_ends([negative, positive], ImageRight::PatientRight),
            SacrumTop::Front {
                sac_l: negative,
                sac_r: positive
            }
        );
        assert_eq!(
            named_ends([negative, positive], ImageRight::Anterior),
            SacrumTop::Side {
                sac_a: positive,
                sac_p: negative
            }
        );
        assert_eq!(
            named_ends([negative, positive], ImageRight::Posterior),
            SacrumTop::Side {
                sac_a: negative,
                sac_p: positive
            }
        );
    }

    #[test]
    fn the_plateau_fits_blurred_walls_and_is_absent_without_support() {
        let (rows, cols) = (81, 121);
        let step = 0.5;
        let mut values = Vec::new();
        let edge = |distance| 0.5 + 0.5 * libm::erf(distance / (libm::sqrt(2.0) * 0.6));
        for row in 0..rows {
            for col in 0..cols {
                let x = (f64::from(col) - 60.0) * step;
                let y = f64::from(row) * step;
                // Analytic plateau y = 10 + 0.1x, bounded by x = ±18,
                // with each edge blurred by a 0.6 mm Gaussian.
                values.push(
                    (0.1 + 0.8 * edge(y - 10.0 - 0.1 * x) * edge(18.0 - x) * edge(x + 18.0)) as f32,
                );
            }
        }
        let mut strip = Strip {
            film: FilmKind::Front,
            rows,
            cols,
            row_step: step,
            col_step: step,
            scale_assumed: false,
            values,
            valid: vec![1; (rows * cols) as usize],
            layout: (0..rows)
                .map(|row| StripRow {
                    centre: Px::new(300.0 + f64::from(row), 200.0),
                    across: Px::new(0.0, -1.0),
                    along: Px::new(1.0, 0.0),
                    curvature: 0.0,
                    half_width: 16.0,
                    z: -f64::from(row) * step,
                })
                .collect(),
            image_right: Some(ImageRight::PatientLeft),
            polarity_hint: Some(Polarity::BoneBright),
            evidence: StripEvidence::default(),
        };
        let plateau = Some(EndplateOnStrip {
            row: 21.0,
            tilt: libm::atan(0.1),
        });
        let Some(SacrumTop::Front { sac_l, sac_r }) = place(&strip, plateau, Polarity::BoneBright)
        else {
            panic!("supported plateau");
        };
        // At x = ±18 the plateau is y = 10 ±1.8 mm. The crop's
        // placement maps (x,y) to film (300+2y, 200-2x). Allow 1 mm
        // (2 film pixels) for the sampled Gaussian edge intersections.
        for (mark, want) in [
            (sac_l, Px::new(323.6, 164.0)),
            (sac_r, Px::new(316.4, 236.0)),
        ] {
            assert!(
                libm::hypot(mark.at.x() - want.x(), mark.at.y() - want.y()) < 2.0,
                "{:?}, expected {want:?}",
                mark.at
            );
            assert!(!mark.inferred);
        }
        assert_eq!(place(&strip, None, Polarity::BoneBright), None);
        strip.values.fill(0.1);
        assert_eq!(place(&strip, plateau, Polarity::BoneBright), None);
    }
}
