//! A designer-authored line of wall with doorway openings. The scene stores
//! only endpoints and openings; ownerless `WaveWall` children are rebuilt as
//! derived engine data, keeping prefabs draggable without scripts.

use godot::classes::notify::Node3DNotification;
use godot::classes::{Engine, INode3D, Material, Node, Node3D};
use godot::prelude::*;

use super::solid::{WaveSolid, warnings_from_level};
use super::wall::WaveWall;
use crate::level_plan;

const SEG_PREFIX: &str = "RunSeg";
const GENERATED_META: &str = "_unseeing_wave_run_segment";

#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct WaveRun {
    /// First endpoint in the parent's local X/Z plane.
    #[export(range = (-60.0, 60.0, 0.1, suffix = " m"))]
    #[var(get = get_from, set = set_from)]
    #[init(val = Vector2::ZERO)]
    from: Vector2,
    /// Second endpoint in the parent's local X/Z plane.
    #[export(range = (-60.0, 60.0, 0.1, suffix = " m"))]
    #[var(get = get_to, set = set_to)]
    #[init(val = Vector2::new(4.0, 0.0))]
    to: Vector2,
    /// Doorways as `(absolute axis coordinate, width)` pairs. Width is a
    /// magnitude and extends toward the run's increasing coordinate.
    #[export]
    #[var(get = get_openings, set = set_openings)]
    openings: PackedVector2Array,
    material: Option<Gd<Material>>,
    own_warnings: PackedStringArray,
    transform_warning: Option<String>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WaveRun {
    fn ready(&mut self) {
        self.absorb_planar_transform();
        self.rebuild();
        self.base_mut().set_notify_local_transform(true);
    }

    fn on_notification(&mut self, what: Node3DNotification) {
        if matches!(what, Node3DNotification::LOCAL_TRANSFORM_CHANGED)
            && self.base().get_transform() != Transform3D::IDENTITY
        {
            self.absorb_planar_transform();
            self.rebuild();
        }
    }

    fn get_configuration_warnings(&self) -> PackedStringArray {
        let mut warnings = self.own_warnings.clone();
        let level_warnings = warnings_from_level(&self.base().clone().upcast::<Node>());
        for warning in level_warnings.as_slice() {
            warnings.push(warning);
        }
        warnings
    }
}

#[godot_api]
impl WaveRun {
    #[func]
    fn set_from(&mut self, value: Vector2) {
        self.from = value;
        self.rebuild_if_ready();
    }

    #[func]
    fn get_from(&self) -> Vector2 {
        self.from
    }

    #[func]
    fn set_to(&mut self, value: Vector2) {
        self.to = value;
        self.rebuild_if_ready();
    }

    #[func]
    fn get_to(&self) -> Vector2 {
        self.to
    }

    #[func]
    fn set_openings(&mut self, value: PackedVector2Array) {
        self.openings = value;
        self.rebuild_if_ready();
    }

    #[func]
    fn get_openings(&self) -> PackedVector2Array {
        self.openings.clone()
    }

    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        INode3D::get_configuration_warnings(self)
    }

    pub(crate) fn set_material(&mut self, material: &Gd<Material>) {
        self.material = Some(material.clone());
        for child in self.base().get_children().iter_shared() {
            if let Ok(mut wall) = child.try_cast::<WaveWall>() {
                WaveSolid::set_material(&mut *wall.bind_mut(), material);
            }
        }
    }

    fn rebuild_if_ready(&mut self) {
        if self.base().is_inside_tree() {
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.clear_segments();
        let openings: Vec<(f64, f64)> = self
            .openings
            .as_slice()
            .iter()
            .map(|v| (v.x as f64, v.y as f64))
            .collect();
        let plan = level_plan::run_segments(self.from, self.to, &openings);
        self.own_warnings.clear();
        if let Some(warning) = self.transform_warning.as_ref() {
            self.own_warnings.push(warning.as_str());
        }
        if let Some(complaint) = plan.complaint {
            self.own_warnings.push(complaint.as_str());
            if !Engine::singleton().is_editor_hint() {
                godot_warn!("{}", complaint);
            }
        }
        for (index, segment) in plan.segments.into_iter().enumerate() {
            let mut wall = WaveWall::new_alloc();
            wall.set_name(&format!("{SEG_PREFIX}{}", index + 1));
            wall.set_meta(GENERATED_META, &true.to_variant());
            wall.bind_mut().set_length(segment.length as f64);
            wall.set_position(Vector3::new(segment.center.x, 0.0, segment.center.y));
            if segment.vertical {
                wall.set_rotation(Vector3::new(0.0, std::f32::consts::FRAC_PI_2, 0.0));
            }
            if let Some(material) = self.material.as_ref() {
                WaveSolid::set_material(&mut *wall.bind_mut(), material);
            }
            self.base_mut().add_child(&wall);
        }
        self.base_mut()
            .call_deferred("update_configuration_warnings", &[]);
    }

    fn clear_segments(&mut self) {
        let doomed: Vec<Gd<Node>> = self
            .base()
            .get_children()
            .iter_shared()
            .filter(|child| child.has_meta(GENERATED_META))
            .collect();
        for child in doomed {
            self.base_mut().remove_child(&child);
            child.free();
        }
    }

    fn absorb_planar_transform(&mut self) {
        let transform = self.base().get_transform();
        if transform == Transform3D::IDENTITY {
            return;
        }
        let map = |point: Vector2| {
            let mapped = transform * Vector3::new(point.x, 0.0, point.y);
            Vector2::new(mapped.x, mapped.z)
        };
        let old_from = self.from;
        let old_to = self.to;
        let old_vertical = (old_to.y - old_from.y).abs() > (old_to.x - old_from.x).abs();
        let new_from = map(old_from);
        let new_to = map(old_to);
        let new_vertical = (new_to.y - new_from.y).abs() > (new_to.x - new_from.x).abs();
        let mut mapped_openings = PackedVector2Array::new();
        for opening in self.openings.as_slice() {
            let width = opening.y.abs();
            let start = if old_vertical {
                Vector2::new(old_from.x, opening.x)
            } else {
                Vector2::new(opening.x, old_from.y)
            };
            let end = if old_vertical {
                Vector2::new(old_from.x, opening.x + width)
            } else {
                Vector2::new(opening.x + width, old_from.y)
            };
            let mapped_start = map(start);
            let mapped_end = map(end);
            let a = if new_vertical {
                mapped_start.y
            } else {
                mapped_start.x
            };
            let b = if new_vertical {
                mapped_end.y
            } else {
                mapped_end.x
            };
            mapped_openings.push(Vector2::new(a.min(b), (b - a).abs()));
        }
        self.from = new_from;
        self.to = new_to;
        self.openings = mapped_openings;

        let up = transform.basis.col_b();
        let cannot_represent = transform.origin.y.abs() > 1e-4
            || (up.length() - 1.0).abs() > 1e-4
            || up.normalized().dot(Vector3::UP).abs() < 0.9999
            || transform.basis.col_a().y.abs() > 1e-4
            || transform.basis.col_c().y.abs() > 1e-4;
        self.base_mut().set_transform(Transform3D::IDENTITY);
        if cannot_represent {
            let warning = "WaveRun: Y translation or tilt cannot be represented by planar X/Z endpoints — the planar projection was kept and height/tilt were discarded.";
            self.transform_warning = Some(warning.to_string());
            if !Engine::singleton().is_editor_hint() {
                godot_warn!("{}", warning);
            }
        }
    }
}
