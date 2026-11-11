//! About 1 mm of noise added to true marks, standing in for a person's
//! clicks.
//!
//! Each point moves by an independent normal step along each film axis,
//! with a standard deviation of 1 mm by default (in pixels, at the film's
//! scale), drawn in the order the marks are listed. Visibility is kept: a
//! click on a point the film does not show is still noisy, and still
//! flagged not visible.

use crate::seeded::Stream;
use shapes::made_up::{FilmAnswer, TrueMark, TrueMarks, TrueVertebra};
use shapes::pixels::Px;

/// The default size of a click's error, mm, per axis.
pub const CLICK_SD_MM: f64 = 1.0;

/// One point moved by a click's error.
fn clicked(mark: TrueMark, sd_px: f64, stream: &mut Stream) -> TrueMark {
    let dx = stream.normal(0.0, sd_px);
    let dy = stream.normal(0.0, sd_px);
    TrueMark {
        at: Px::new(mark.at.x() + dx, mark.at.y() + dy),
        visible: mark.visible,
    }
}

/// Every true mark of `answer`'s vertebrae moved by a normal error of
/// `sd_mm` per axis, from the click stream of `seed`.
pub fn clicked_vertebrae(answer: &FilmAnswer, sd_mm: f64, seed: u64) -> Vec<TrueVertebra> {
    let mm_per_px = answer.geometry.mm_per_px.unwrap_or(1.0);
    let sd_px = sd_mm / mm_per_px;
    let mut stream = Stream::new(&format!("clicks/{}", answer.kind.name()), seed);
    let mut click = |mark: TrueMark| clicked(mark, sd_px, &mut stream);
    answer
        .vertebrae
        .iter()
        .map(|vertebra| TrueVertebra {
            level: vertebra.level,
            marks: match vertebra.marks {
                TrueMarks::Front {
                    sup_l,
                    sup_r,
                    inf_l,
                    inf_r,
                    ped_l,
                    ped_r,
                } => TrueMarks::Front {
                    sup_l: click(sup_l),
                    sup_r: click(sup_r),
                    inf_l: click(inf_l),
                    inf_r: click(inf_r),
                    ped_l: click(ped_l),
                    ped_r: click(ped_r),
                },
                TrueMarks::Side {
                    sup_a,
                    sup_p,
                    inf_a,
                    inf_p,
                } => TrueMarks::Side {
                    sup_a: click(sup_a),
                    sup_p: click(sup_p),
                    inf_a: click(inf_a),
                    inf_p: click(inf_p),
                },
                TrueMarks::Bending { sup, inf } => TrueMarks::Bending {
                    sup: sup.map(&mut click),
                    inf: inf.map(&mut click),
                },
            },
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::answer_key::corners_of;
    use crate::make_up_answer_key;
    use shapes::made_up::{MadeUpRecipe, RecipeName, Tier};

    #[test]
    fn clicks_land_about_a_millimetre_off_and_keep_their_flags() {
        let key = make_up_answer_key(&MadeUpRecipe {
            name: RecipeName::Random,
            seed: 4,
            tier: Tier::T2,
        })
        .expect("a key");
        let square = |v: f64| v * v;
        let mut squares = Vec::new();
        for answer in &key.films {
            let mm = answer.geometry.mm_per_px.expect("a scale");
            let clicks = clicked_vertebrae(answer, CLICK_SD_MM, 11);
            for (truth, click) in answer.vertebrae.iter().zip(&clicks) {
                assert_eq!(truth.level, click.level);
                for (t, c) in corners_of(&truth.marks)
                    .iter()
                    .zip(corners_of(&click.marks))
                {
                    assert_eq!(t.visible, c.visible);
                    squares.push(square((c.at.x() - t.at.x()) * mm));
                    squares.push(square((c.at.y() - t.at.y()) * mm));
                }
            }
        }
        // The spread of several hundred steps is 1 mm within a few percent.
        let sd = (squares.iter().sum::<f64>() / squares.len() as f64).sqrt();
        assert!((sd - 1.0).abs() < 0.08, "{sd}");
        // The same seed clicks the same way, another seed otherwise.
        let answer = &key.films[0];
        assert_eq!(
            clicked_vertebrae(answer, 1.0, 11),
            clicked_vertebrae(answer, 1.0, 11)
        );
        assert_ne!(
            clicked_vertebrae(answer, 1.0, 11),
            clicked_vertebrae(answer, 1.0, 12)
        );
        // No noise, no change.
        assert_eq!(clicked_vertebrae(answer, 0.0, 11), answer.vertebrae);
    }
}
