//! Passive presentation snapshots, independent of animation state.
use std::ops::Range;

use bevy::{ecs::component::Component, math::Vec2};

use crate::SpinalFrame;
use crate::runtime::{SpinalDraw, SpinalVertex};

/// Marks an instance whose frame is supplied by a recording.
///
/// Add this with `SpinalInstance` and `SpinalFrame`. Animation preparation and
/// advancement leave it alone; the ordinary renderer still draws its frame.
/// The caller owns asset identity, transform, appearance and visibility.
#[derive(Component, Debug, Default)]
pub struct SpinalRecordedFrame;

/// One indexed vertex in unflipped skeleton space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpinalRecordedVertex {
    /// Solved local position.
    pub position: Vec2,
    /// Normalised atlas coordinates.
    pub uv: Vec2,
}

/// One draw in the renderer's original order.
#[derive(Clone, Debug, PartialEq)]
pub struct SpinalRecordedDraw {
    /// Atlas page in the trusted skeleton asset.
    pub page_ordinal: usize,
    /// Allowed vertex range for this draw's indices.
    pub vertices: Range<usize>,
    /// Triangle index range.
    pub indices: Range<usize>,
    /// Unpremultiplied sRGBA slot colour, before instance modulation.
    pub color: [f32; 4],
}

/// Owned geometry for a single solved frame, including diagnostic markers.
///
/// This is presentation data, not a serialisation format or animation checkpoint.
/// Validate external allocation limits before constructing its vectors. Import
/// rejects nonfinite values and inconsistent or excessive geometry atomically.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpinalFrameSnapshot {
    /// Indexed geometry shared across draws.
    pub vertices: Vec<SpinalRecordedVertex>,
    /// Global indices into `vertices`.
    pub indices: Vec<u32>,
    /// Ordered draws.
    pub draws: Vec<SpinalRecordedDraw>,
    /// Local diagnostic marker positions.
    pub issue_points: Vec<Vec2>,
    /// False means the renderer must draw nothing.
    pub ready: bool,
}

/// Invalid or excessive recorded geometry. The previous frame is retained.
#[derive(Clone, Copy, Debug, thiserror::Error)]
#[error("invalid or excessive recorded Spinal geometry")]
pub struct InvalidRecordedFrame;

impl SpinalFrameSnapshot {
    fn valid(&self) -> bool {
        // Extraction copies each draw's ranges. Bound expanded work as well as
        // storage, even when several draws deliberately reuse the same ranges.
        let ranges_fit = |vertices: bool, limit: usize| {
            self.draws
                .iter()
                .try_fold(0usize, |total, d| {
                    let range = if vertices { &d.vertices } else { &d.indices };
                    total
                        .checked_add(range.end.checked_sub(range.start)?)
                        .filter(|total| *total <= limit)
                })
                .is_some()
        };
        self.vertices.len() <= 16_384
            && self.indices.len() <= 65_536
            && self.draws.len() <= 512
            && self.issue_points.len() <= 256
            && ranges_fit(true, 16_384)
            && ranges_fit(false, 65_536)
            && self
                .vertices
                .iter()
                .all(|v| v.position.is_finite() && v.uv.is_finite())
            && self.issue_points.iter().all(|p| p.is_finite())
            && self.draws.iter().all(|d| {
                d.page_ordinal < 256
                    && !d.vertices.is_empty()
                    && !d.indices.is_empty()
                    && d.color
                        .iter()
                        .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
                    && self.vertices.get(d.vertices.clone()).is_some()
                    && self.indices.get(d.indices.clone()).is_some_and(|indices| {
                        indices.len() % 3 == 0
                            && indices.iter().all(|i| d.vertices.contains(&(*i as usize)))
                    })
            })
    }
}

impl SpinalFrame {
    /// Copies the actual solved geometry, including draw colour and order.
    #[must_use]
    pub fn snapshot(&self) -> SpinalFrameSnapshot {
        SpinalFrameSnapshot {
            vertices: self
                .vertices
                .iter()
                .map(|v| SpinalRecordedVertex {
                    position: v.position,
                    uv: v.uv,
                })
                .collect(),
            indices: self.indices.clone(),
            draws: self
                .draws
                .iter()
                .map(|d| SpinalRecordedDraw {
                    page_ordinal: d.page_ordinal,
                    vertices: d.vertices.clone(),
                    indices: d.indices.clone(),
                    color: d.color,
                })
                .collect(),
            issue_points: self.issue_points.clone(),
            ready: self.ready,
        }
    }

    /// Replaces presentation geometry after validation, preserving the old frame
    /// on error. Use `SpinalRecordedFrame` to prevent animation overwriting it.
    pub fn set_recorded(
        &mut self,
        snapshot: SpinalFrameSnapshot,
    ) -> Result<(), InvalidRecordedFrame> {
        if !snapshot.valid() {
            return Err(InvalidRecordedFrame);
        }
        self.vertices = snapshot
            .vertices
            .into_iter()
            .map(|v| SpinalVertex {
                position: v.position,
                uv: v.uv,
            })
            .collect();
        self.indices = snapshot.indices;
        self.draws = snapshot
            .draws
            .into_iter()
            .map(|d| SpinalDraw {
                page_ordinal: d.page_ordinal,
                vertices: d.vertices,
                indices: d.indices,
                color: d.color,
            })
            .collect();
        self.issue_points = snapshot.issue_points;
        self.ready = snapshot.ready;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }
}
