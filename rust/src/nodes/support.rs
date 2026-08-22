use crate::support_motion::MotionPhase;

pub const CONTROLLED_ACTOR_LAYER: u32 = 1 << 1;
pub const AIRBORNE_ACTOR_LAYER: u32 = 1 << 2;
pub const ALL_LAYERS: u32 = u32::MAX;
pub const FLOOR_SNAP_M: f32 = 0.10;
pub const FLOOR_MAX_ANGLE_RAD: f32 = std::f32::consts::FRAC_PI_4;
pub const SAFE_MARGIN_M: f32 = 0.001;
pub const MAX_SLIDES: i32 = 6;
pub const MOTION_RESULT_MAX_CONTACTS: i32 = 6;
pub const SNAP_PROBE_MAX_CONTACTS: i32 = 4;
pub const PLATFORM_LAYERS: u32 = 0;

pub fn collision_pair(phase: MotionPhase) -> (u32, u32) {
    match phase {
        MotionPhase::Controlled => (CONTROLLED_ACTOR_LAYER, ALL_LAYERS & !AIRBORNE_ACTOR_LAYER),
        MotionPhase::Airborne { .. } => (
            AIRBORNE_ACTOR_LAYER,
            ALL_LAYERS & !(CONTROLLED_ACTOR_LAYER | AIRBORNE_ACTOR_LAYER),
        ),
    }
}

pub fn is_actor_layer(layer: u32) -> bool {
    layer & (CONTROLLED_ACTOR_LAYER | AIRBORNE_ACTOR_LAYER) != 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::support_motion::{FiniteVelocity, MotionPhase, PlanarVelocity};

    #[test]
    fn collision_pairs_exclude_airborne_actors_without_hiding_world_layers() {
        assert_eq!(CONTROLLED_ACTOR_LAYER, 2);
        assert_eq!(AIRBORNE_ACTOR_LAYER, 4);
        assert_eq!(collision_pair(MotionPhase::Controlled), (2, 4_294_967_291));

        let airborne = MotionPhase::Airborne {
            planar_velocity_mps: PlanarVelocity::try_new(1.0, -2.0).unwrap(),
            vertical_velocity_mps: FiniteVelocity::try_new(-3.0).unwrap(),
        };
        assert_eq!(collision_pair(airborne), (4, 4_294_967_289));
        assert!(is_actor_layer(2));
        assert!(is_actor_layer(4));
        assert!(is_actor_layer(6));
        assert!(!is_actor_layer(0));
        assert!(!is_actor_layer(1));
        assert!(!is_actor_layer(8));
    }

    #[test]
    fn solver_constants_keep_the_authored_godot_contract() {
        assert_eq!(ALL_LAYERS, 4_294_967_295);
        assert_eq!(FLOOR_SNAP_M.to_bits(), 0.10_f32.to_bits());
        assert_eq!(FLOOR_MAX_ANGLE_RAD.to_bits(), 0x3f49_0fdb);
        assert_eq!(SAFE_MARGIN_M.to_bits(), 0.001_f32.to_bits());
        assert_eq!(MAX_SLIDES, 6);
        assert_eq!(MOTION_RESULT_MAX_CONTACTS, 6);
        assert_eq!(SNAP_PROBE_MAX_CONTACTS, 4);
        assert_eq!(PLATFORM_LAYERS, 0);
    }
}
