//! One first-peak, truncated-window centroid per fixed landmark channel.

use crate::{grid::Grid, layers::softmax_mean::softmax_mean};
use shapes::errors::{EngineError, ErrorCode};

/// Decodes a channel in model pixel centres. Borders have no repeated mass.
pub fn channel(maps: &Grid, channel: usize) -> Result<[f32; 2], EngineError> {
    if maps.channels() != 20 || channel >= 20 || maps.rows() == 0 || maps.columns() == 0 {
        return Err(invalid());
    }
    let mut peak = [0, 0];
    let mut largest = f32::NEG_INFINITY;
    for y in 0..maps.rows() {
        for (x, &value) in maps.row(channel, y).iter().enumerate() {
            if !value.is_finite() {
                return Err(invalid());
            }
            if value > largest {
                largest = value;
                peak = [x, y];
            }
        }
    }
    let mut scores = [0.0; 49];
    let mut points = [[0.0; 2]; 49];
    let mut count = 0;
    for dy in -3isize..=3 {
        for dx in -3isize..=3 {
            let x = peak[0] as isize + dx;
            let y = peak[1] as isize + dy;
            if x >= 0 && y >= 0 && x < maps.columns() as isize && y < maps.rows() as isize {
                scores[count] = maps.at(channel, y as usize, x as usize);
                points[count] = [x as f32, y as f32];
                count += 1;
            }
        }
    }
    let mean = softmax_mean(&scores[..count], &points[..count]);
    let point = mean.map(|v| (v + 0.5) * 4.0 - 0.5);
    if !point.iter().all(|v| v.is_finite()) {
        return Err(invalid());
    }
    Ok(point)
}

fn invalid() -> EngineError {
    EngineError::new(
        ErrorCode::EngineOutOfSync,
        "the side landmark map or point is not finite and valid",
    )
}
