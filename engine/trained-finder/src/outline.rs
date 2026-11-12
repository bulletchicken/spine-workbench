//! The plain shapes the stages hand to one another.
//!
//! Every position here is a fraction of the picture the network was
//! shown: 0 at its left or top edge, 1 at its right or bottom edge. The
//! picture is the whole film on the first reading and the framed crop on
//! the second; [`crate::framing`] maps the crop's fractions back to the
//! film. A quad's corners always go round the outline: upper left, upper
//! right, lower right, lower left, as the picture shows them.

/// A point as `[across, down]`.
pub type Point = [f64; 2];

/// Four corners going round: upper left, upper right, lower right, lower
/// left.
pub type Quad = [Point; 4];

/// One local maximum of the network's heat map, read as an outline.
#[derive(Clone, Debug, PartialEq)]
pub struct Peak {
    /// The network's own score for this vertebra, 0 to 1.
    pub score: f64,
    /// Where the network puts the vertebra's centre.
    pub centre: Point,
    pub corners: Quad,
    /// The heat map cell the peak sits in, as `[column, row]`.
    pub cell: [u32; 2],
}

/// What one reading of a picture gives.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// Every local maximum scoring at least 0.05, best first, ties by
    /// cell in row order. The chain and the framing choose from these.
    pub peaks: Vec<Peak>,
    /// The best thirty peaks at or above the trained threshold, each kept
    /// only if it overlaps no better one by half or more. Only the
    /// framing gate reads these.
    pub outlines: Vec<Peak>,
}

/// One vertebra the chain settled on.
#[derive(Clone, Debug, PartialEq)]
pub struct ChainLink {
    /// 0 for T1 up to 16 for L5.
    pub level_index: usize,
    /// The network did not see this one: it was placed halfway between
    /// its two neighbours.
    pub filled_in: bool,
    pub corners: Quad,
    pub centre: Point,
    /// The peak's score; 0 for a filled-in link.
    pub score: f64,
    /// The peak's place in [`Reading::peaks`]; `None` for a filled-in
    /// link.
    pub peak_index: Option<usize>,
}
