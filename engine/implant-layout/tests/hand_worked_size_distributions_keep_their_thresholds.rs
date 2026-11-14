//! Synthetic distributions with answers calculated before running the rules.

use implant_layout::screws::sizing::{GridSize, choose_diameter, choose_length, length_grid};

fn diameter_grid() -> Vec<GridSize> {
    [4.0, 5.0, 6.0, 7.0]
        .into_iter()
        .map(|mm| GridSize {
            mm,
            sources: vec!["synthetic-test-grid".into()],
        })
        .collect()
}

#[test]
fn constant_and_half_widths_choose_the_lower_tail_not_the_upper_tail() {
    let grid = diameter_grid();
    // Constant W=5, k=1.25: d<=6.25, so the answer is 6.
    let constant = choose_diameter(&[5.0; 10], &grid, 1.25, 0.1).unwrap();
    assert_eq!(constant.mm, Some(6.0));
    assert!(constant.also_seen.is_empty());
    // Half W=4, half W=6: d=5 passes all versions; d=6 fails half.
    // Individual choices are 5 and 7, each in half the versions.
    let half = choose_diameter(
        &[4.0, 4.0, 4.0, 4.0, 4.0, 6.0, 6.0, 6.0, 6.0, 6.0],
        &grid,
        1.25,
        0.1,
    )
    .unwrap();
    assert_eq!(half.mm, Some(5.0));
    assert_eq!(half.also_seen, vec![7.0]);
    // All W=3: even the smallest grid size needs W>=3.2.
    let none = choose_diameter(&[3.0; 10], &grid, 1.25, 0.1).unwrap();
    assert_eq!(none.mm, None);
    assert!(none.also_seen.is_empty());
}

#[test]
fn constant_chords_round_down_after_the_share_and_margin_caps() {
    let grid = length_grid();
    // min(.8*42,42-4)=33.6, whose lower catalogue neighbour is 32.5.
    assert_eq!(
        choose_length(&[42.0; 10], &grid, 0.8, 4.0, 0.1).unwrap().mm,
        Some(32.5)
    );
    // With f=1 the 4 mm margin controls: 37-4=33 -> 32.5.
    assert_eq!(
        choose_length(&[37.0; 10], &grid, 1.0, 4.0, 0.1).unwrap().mm,
        Some(32.5)
    );
    // A child's central factor .65 gives 26 -> 25; the shown .5 and
    // .8 alternatives give 20 -> no size, and 32 -> 30 respectively.
    for (factor, expected) in [(0.5, None), (0.65, Some(25.0)), (0.8, Some(30.0))] {
        assert_eq!(
            choose_length(&[40.0; 10], &grid, factor, 4.0, 0.1)
                .unwrap()
                .mm,
            expected
        );
    }
}

#[test]
fn a_chord_spread_uses_ninety_percent_before_rounding() {
    let grid = length_grid();
    // Nine limits 33.6, one limit 28: 32.5 qualifies in exactly 90%.
    // Per-version lengths 32.5 and 27.5 give the latter as also-seen.
    let mut chords = [42.0; 10];
    chords[0] = 35.0;
    let choice = choose_length(&chords, &grid, 0.8, 4.0, 0.1).unwrap();
    assert_eq!(choice.mm, Some(32.5));
    assert_eq!(choice.also_seen, vec![27.5]);
    // Two short versions leave only 80%, so the answer drops to 27.5.
    chords[1] = 35.0;
    let choice = choose_length(&chords, &grid, 0.8, 4.0, 0.1).unwrap();
    assert_eq!(choice.mm, Some(27.5));
    assert_eq!(choice.also_seen, vec![32.5]);
}
