//! Recorded presentation validation and round-trip contract.
use bevy::math::Vec2;
use bevy_spinal::{SpinalFrame, SpinalFrameSnapshot, SpinalRecordedDraw, SpinalRecordedVertex};

fn snapshot() -> SpinalFrameSnapshot {
    SpinalFrameSnapshot {
        vertices: vec![
            SpinalRecordedVertex {
                position: Vec2::ZERO,
                uv: Vec2::ZERO,
            },
            SpinalRecordedVertex {
                position: Vec2::X,
                uv: Vec2::X,
            },
            SpinalRecordedVertex {
                position: Vec2::Y,
                uv: Vec2::Y,
            },
        ],
        indices: vec![0, 1, 2],
        draws: vec![SpinalRecordedDraw {
            page_ordinal: 0,
            vertices: 0..3,
            indices: 0..3,
            color: [0.5, 1., 1., 0.25],
        }],
        issue_points: vec![],
        ready: true,
    }
}

#[test]
fn recorded_frame_roundtrips_draw_order_color_and_geometry() {
    let source = snapshot();
    let mut frame = SpinalFrame::default();
    frame.set_recorded(source.clone()).unwrap();
    assert_eq!(frame.snapshot(), source);
    assert_eq!(frame.triangles().count(), 1);
    assert!(frame.is_ready());
}

#[test]
fn invalid_recorded_frame_does_not_replace_the_previous_frame() {
    let source = snapshot();
    let mut frame = SpinalFrame::default();
    frame.set_recorded(source.clone()).unwrap();
    for bad in [0, 1, 2, 3] {
        let mut invalid = source.clone();
        match bad {
            0 => invalid.indices[2] = 3,
            1 => invalid.vertices[0].position.x = f32::NAN,
            2 => invalid.draws[0].color[0] = f32::INFINITY,
            _ => invalid.draws[0].indices.end = 2,
        }
        assert!(frame.set_recorded(invalid).is_err());
        assert_eq!(frame.snapshot(), source);
    }
}

#[test]
fn repeated_draw_ranges_cannot_expand_past_the_frame_budget() {
    let mut invalid = snapshot();
    invalid.vertices.resize(16_384, invalid.vertices[0]);
    invalid.draws[0].vertices = 0..16_384;
    invalid.draws.push(invalid.draws[0].clone());
    assert!(SpinalFrame::default().set_recorded(invalid).is_err());
}

#[test]
fn empty_draw_ranges_are_rejected_before_the_renderer() {
    let mut invalid = snapshot();
    invalid.draws[0].indices = 0..0;
    assert!(SpinalFrame::default().set_recorded(invalid).is_err());
}
