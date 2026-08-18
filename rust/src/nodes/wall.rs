//! The wall — the one solid that is more than a shape. A level is authored
//! by dragging these around the editor: place one, rotate it, stretch its
//! `length` knob, and it builds its own outline mesh and collider in
//! `_ready` from nothing but transform and knob, so what the editor shows is
//! exactly what the waves will strike.
//!
//! A wall is special because it OCCLUDES. Its centerline is the level's
//! technical contract: the sight shaders count wall crossings to decide
//! what a source may light and what the hero may hear, so a wall's geometry
//! is physics, not decoration. That is why the axis law is enforced here —
//! a free-hand rotation snaps to the nearest quarter turn on entering the
//! tree and whenever its global placement changes, from
//! [`level_plan::normalized_wall_basis`]'s exact 0/±1 columns. The node's
//! scale is discarded with the same stroke: `length` is the one size knob.
//! The free shapes with no such contract live in [`super::props`].
//!
//! Both the snap and the centerline it feeds are read in WORLD space, and
//! that is not a detail: a wall is authored inside whatever room prefab
//! carries it, so the transform it draws under is its ancestors' as much as
//! its own. Snapping the local basis under a turned room would leave the
//! box drawn down one axis and the occluder derived down the other — sound
//! passing through a wall the eye is shown, and stopping at air it is not.
//!
//! The world skin arrives from the level root ([`super::level`]) — the
//! single injection point — not per-node; a bare node in the editor simply
//! shows its plain box.

use godot::classes::{
    ArrayMesh, BoxShape3D, Engine, INode3D, InputEvent, Material, Node, Node3D, PhysicsMaterial,
    StaticBody3D,
};
use godot::prelude::*;

use super::solid::{
    self, BOX_ORDINALS, SignFold, Skin, WaveSolid, build_box, clear_limbs, warnings_from_level,
};
use crate::level_plan;
use crate::render;

const ANCESTOR_TRANSFORM_WARNING: &str = "WaveWall: an ancestor transform is singular, \
non-finite, or too large to represent, so this wall cannot normalize its global basis. Repair \
zero scale or non-finite values, or reduce the ancestor's scale or position; it will snap \
automatically when the ancestor is representable.";
const OWN_TRANSFORM_WARNING: &str = "WaveWall: its transform contained NaN or infinity. Every \
finite position lane was preserved and the rest was restored from its last valid placement. Move \
the wall once to acknowledge and clear this warning.";
const BODY_CONTRACT_WARNING: &str = "WaveWall: collision priority was non-finite, too large, or \
not positive. Its last valid value (or the default before one existed) was restored; edit \
Collision Priority to clear this warning.";
const LENGTH_WARNING: &str = "WaveWall: length was non-finite or too large for Godot geometry. \
The last finite wall length was preserved; enter a finite value that fits the editor to clear this \
warning.";
const WALL_BODY_NAME: &str = "WaveBody";
const WALL_BODY_META: &str = "_unseeing_wave_wall_body";
const LEGACY_WALL_LIMBS: [&str; 2] = [solid::SKIN_NAME, solid::COLLIDER_NAME];

/// One wall segment: an axis-snapped box, `length` meters of centerline
/// padded by a wall half-thickness each way, floor to ceiling. The node
/// stands on the floor at the centerline's midpoint; the box rises from
/// it.
#[derive(GodotClass)]
#[class(tool, init, base=Node3D)]
pub struct WaveWall {
    /// Centerline length in meters — the designer's one size knob, and a
    /// magnitude: a negative reading folds at the knob ([`SignFold`]).
    #[export(range = (0.3, 30.0, 0.1, or_greater, suffix = " m"))]
    #[var(get = get_length, set = set_length)]
    #[init(val = 4.0)]
    length: f64,
    /// Physics layer of the private exact body. WaveWall is intentionally an
    /// authoring datum rather than a dummy StaticBody proxy.
    #[export(flags_3d_physics)]
    #[var(get = get_collision_layer, set = set_collision_layer)]
    #[init(val = 1)]
    collision_layer: u32,
    /// Physics mask of the private exact body.
    #[export(flags_3d_physics)]
    #[var(get = get_collision_mask, set = set_collision_mask)]
    #[init(val = 1)]
    collision_mask: u32,
    /// Collision priority forwarded to the private exact body.
    #[export(range = (0.01, 100.0, 0.01, or_greater))]
    #[var(get = get_collision_priority, set = set_collision_priority)]
    #[init(val = 1.0)]
    collision_priority: f64,
    /// Whether pointer/ray picking may reach this wall's input signals.
    #[export]
    #[var(get = is_ray_pickable, set = set_ray_pickable)]
    #[init(val = true)]
    ray_pickable: bool,
    /// Preserve Godot's drag-capture choice on the generated body.
    #[export]
    #[var(get = get_input_capture_on_drag, set = set_input_capture_on_drag)]
    input_capture_on_drag: bool,
    /// Optional surface friction/bounce resource for the generated body.
    #[export]
    #[var(get = get_physics_material_override, set = set_physics_material_override)]
    physics_material_override: Option<Gd<PhysicsMaterial>>,
    skin: Skin,
    fold: SignFold,
    body: Option<Gd<StaticBody3D>>,
    mesh: Option<Gd<ArrayMesh>>,
    shape: Option<Gd<BoxShape3D>>,
    normalizing_transform: bool,
    transform_memory: level_plan::WallTransformMemory,
    priority_memory: level_plan::WallPriorityMemory,
    normalization_writes: u64,
    body_transform_writes: u64,
    body_contract_writes: u64,
    transform_warning: Option<String>,
    body_contract_warning: Option<String>,
    length_warning: Option<String>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WaveWall {
    fn ready(&mut self) {
        self.snap_to_axis();
        let editor = Engine::singleton().is_editor_hint();
        let mut base = self.base_mut();
        base.set_process(editor);
        drop(base);
        // a duplicated wall arrives carrying the original's limbs; this
        // build owns the pair, so the ghosts go first
        clear_limbs(self, &LEGACY_WALL_LIMBS);
        self.clear_generated_body();
        let size = level_plan::wall_box(self.length);
        let lift = Vector3::new(0.0, (level_plan::WALL_H * 0.5) as f32, 0.0);
        let built = build_box(size, lift, self.skin.material());
        // The authored WaveWall stays in its prefab hierarchy so the
        // Inspector keeps showing a parent-local placement. Godot cannot
        // represent an exact unit global basis under every oblique,
        // non-uniform parent, though: inverse/recomposition leaves a few
        // low bits of dust. Put the generated strikeable body on the exact
        // canonical world transform instead. Its ordinary local children
        // keep the mesh and PhysicsServer shape on that same body transform;
        // making the CollisionShape itself top-level would double-compose it
        // through StaticBody3D's shape owner.
        let mut body = StaticBody3D::new_alloc();
        body.set_name(WALL_BODY_NAME);
        body.set_meta(WALL_BODY_META, &true.to_variant());
        body.set_as_top_level(true);
        let owner = self.base().clone().upcast::<Object>();
        for (signal, method) in [
            ("input_event", "relay_input_event"),
            ("mouse_entered", "relay_mouse_entered"),
            ("mouse_exited", "relay_mouse_exited"),
        ] {
            let relay = Callable::from_object_method(&owner, method);
            body.connect(signal, &relay);
        }
        body.add_child(&built.skin);
        body.add_child(&built.collider);
        self.base_mut().add_child(&body);
        self.skin.adopt(built.skin);
        self.body = Some(body);
        self.mesh = Some(built.mesh);
        self.shape = Some(built.shape);
        self.sync_body_transform();
        self.sync_body_contract();
        let name = self.base().get_name();
        self.fold.say(Some(name));
        self.say_pending_length_warning();
        self.say_pending_body_contract_warning();
    }

    /// A wall's axis law is global, so editor mode polls the composed
    /// placement once per process frame. Runtime walls are immutable authored
    /// geometry after `_ready`; WaveLevel deliberately does not rederive its
    /// retained paint/occlusion tables every runtime frame, so this adapter
    /// must not move only one half of that contract behind its back. Godot
    /// itself reports one failed affine inverse at the
    /// instant an ancestor is changed to zero scale, before extension code can
    /// observe the edit. Polling keeps this adapter from adding a doomed global
    /// write—or an error storm—afterward: it classifies the state, stores one
    /// repair warning, and waits. The work is a bounded parent transform read
    /// plus exact comparisons; the pure plan decides every nontrivial branch.
    fn process(&mut self, _delta: f64) {
        self.snap_to_axis();
    }

    /// The Scene dock's warning icon for this one wall: first the transform
    /// fault only this boundary can own, then whatever its owning
    /// [`super::level::WaveLevel`] pinned to this path via
    /// [`warnings_from_level`]. A valid prefab outside a level stays empty.
    fn get_configuration_warnings(&self) -> PackedStringArray {
        let mut warnings = PackedStringArray::new();
        if let Some(warning) = self.transform_warning.as_ref() {
            warnings.push(warning.as_str());
        }
        if let Some(warning) = self.body_contract_warning.as_ref() {
            warnings.push(warning.as_str());
        }
        if let Some(warning) = self.length_warning.as_ref() {
            warnings.push(warning.as_str());
        }
        let level_warnings = warnings_from_level(&self.base().clone().upcast::<Node>());
        for warning in level_warnings.as_slice() {
            warnings.push(warning);
        }
        warnings
    }
}

#[godot_api]
impl WaveWall {
    /// Pointer/ray input from the generated physics body, relayed without
    /// exposing that implementation limb to authored scene logic.
    #[signal]
    fn input_event(
        camera: Option<Gd<Node>>,
        event: Option<Gd<InputEvent>>,
        event_position: Vector3,
        normal: Vector3,
        shape_idx: i64,
    );

    #[signal]
    fn mouse_entered();

    #[signal]
    fn mouse_exited();

    #[func]
    fn set_collision_layer(&mut self, value: u32) {
        self.collision_layer = value;
        self.sync_body_contract();
    }

    #[func]
    fn get_collision_layer(&self) -> u32 {
        self.collision_layer
    }

    #[func]
    fn set_collision_mask(&mut self, value: u32) {
        self.collision_mask = value;
        self.sync_body_contract();
    }

    #[func]
    fn get_collision_mask(&self) -> u32 {
        self.collision_mask
    }

    #[func]
    fn set_collision_priority(&mut self, value: f64) {
        let priority = level_plan::plan_wall_priority(value as f32, self.priority_memory);
        self.priority_memory = priority.memory;
        self.collision_priority = f64::from(priority.value);
        self.set_body_contract_warning(priority.warn.then_some(BODY_CONTRACT_WARNING));
        self.sync_body_contract();
    }

    #[func]
    fn get_collision_priority(&self) -> f64 {
        self.collision_priority
    }

    #[func]
    fn set_ray_pickable(&mut self, value: bool) {
        self.ray_pickable = value;
        self.sync_body_contract();
    }

    #[func]
    fn is_ray_pickable(&self) -> bool {
        self.ray_pickable
    }

    #[func]
    fn set_input_capture_on_drag(&mut self, value: bool) {
        self.input_capture_on_drag = value;
        self.sync_body_contract();
    }

    #[func]
    fn get_input_capture_on_drag(&self) -> bool {
        self.input_capture_on_drag
    }

    #[func]
    fn set_physics_material_override(&mut self, value: Option<Gd<PhysicsMaterial>>) {
        self.physics_material_override = value;
        self.sync_body_contract();
    }

    #[func]
    fn get_physics_material_override(&self) -> Option<Gd<PhysicsMaterial>> {
        self.physics_material_override.clone()
    }

    /// Remove only a prior generated physics limb. The readable `WaveBody`
    /// name is not identity: a designer is free to use that name for an
    /// authored child and duplication must preserve it.
    fn clear_generated_body(&mut self) {
        let mut owner = self.base().clone().upcast::<Node>();
        let stale: Vec<Gd<Node>> = owner
            .get_children()
            .iter_shared()
            .filter(|child| {
                child.clone().try_cast::<StaticBody3D>().is_ok()
                    && child.has_meta(WALL_BODY_META)
                    && child
                        .get_meta(WALL_BODY_META)
                        .try_to::<bool>()
                        .unwrap_or(false)
            })
            .collect();
        for body in stale {
            owner.remove_child(&body);
            body.free();
        }
    }

    /// Read-only test/observer door for the number of global normalization
    /// writes this wall has performed. It keeps the live-boundary regression
    /// honest: an oblique parent round-trip may be numerically close rather
    /// than bit-identical, but an unchanged scene must still settle after one
    /// write instead of rewriting every frame.
    #[func]
    fn normalization_writes(&self) -> i64 {
        i64::try_from(self.normalization_writes).unwrap_or(i64::MAX)
    }

    /// Read-only witness for writes to the generated canonical body. The
    /// editor watches every frame, but an unchanged wall must settle: this
    /// count moves only when the exact visual/physics transform actually
    /// changes.
    #[func]
    fn body_transform_writes(&self) -> i64 {
        i64::try_from(self.body_transform_writes).unwrap_or(i64::MAX)
    }

    /// Read-only witness for generated-body physics-contract writes. Repeating
    /// an authored value through its setter must not wake the physics server
    /// or reconnect a PhysicsMaterial.
    #[func]
    fn body_contract_writes(&self) -> i64 {
        i64::try_from(self.body_contract_writes).unwrap_or(i64::MAX)
    }

    /// Relay the generated physics body's inherited CollisionObject3D signal
    /// to the designer-facing WaveWall. The exact event payload is unchanged;
    /// only the private collider implementation is hidden.
    #[func]
    fn relay_input_event(
        &mut self,
        camera: Option<Gd<Node>>,
        event: Option<Gd<InputEvent>>,
        event_position: Vector3,
        normal: Vector3,
        shape_idx: i64,
    ) {
        self.base_mut().emit_signal(
            "input_event",
            &[
                camera.to_variant(),
                event.to_variant(),
                event_position.to_variant(),
                normal.to_variant(),
                shape_idx.to_variant(),
            ],
        );
    }

    #[func]
    fn relay_mouse_entered(&mut self) {
        self.base_mut().emit_signal("mouse_entered", &[]);
    }

    #[func]
    fn relay_mouse_exited(&mut self) {
        self.base_mut().emit_signal("mouse_exited", &[]);
    }

    /// The length knob reshapes the wall live in the editor, mesh and collider
    /// together, on the knob's magnitude. Runtime level geometry is immutable
    /// after ready because its derived paint/occlusion table is intentionally
    /// not rebuilt per frame; a post-ready runtime assignment is ignored.
    /// A wall is three things derived from this one number (a drawn box, a
    /// collider, an occluding centerline) and they answer a minus sign
    /// three different ways, so the sign never gets past here.
    #[func]
    pub(crate) fn set_length(&mut self, length: f64) {
        if !level_plan::authored_geometry_edit_is_live(
            self.base().is_inside_tree(),
            Engine::singleton().is_editor_hint(),
        ) {
            return;
        }
        let plan = level_plan::sanitize_wall_length(length, self.length);
        self.length = if plan.repaired {
            plan.value
        } else {
            self.fold.scalar("length", length)
        };
        self.set_length_warning(plan.repaired.then_some(LENGTH_WARNING));
        let size = level_plan::wall_box(self.length);
        if let Some(mesh) = self.mesh.as_mut() {
            render::paint::resize_box_surface(mesh, size, BOX_ORDINALS);
        }
        if let Some(shape) = self.shape.as_mut() {
            shape.set_size(size);
        }
        let named = self.base().is_inside_tree().then(|| self.base().get_name());
        self.fold.say(named);
    }

    #[func]
    fn get_length(&self) -> f64 {
        self.length
    }

    /// The id this wall carries — the engine-facing read-back of its own
    /// [`Skin`], off the mesh's own `CUSTOM0`, so the suites can hold the
    /// seam law against a scene without binding Rust traits.
    #[func]
    fn oid(&self) -> f64 {
        self.skin.oid()
    }

    /// Read-only engine witness for the exact analytic box frame handed to
    /// the superface/paint pass. It deliberately reconstructs the frame from
    /// [`Self::world_shape`] rather than from the generated limbs, so an
    /// accidental return to the authored container's dusty parent-composed
    /// transform cannot hide behind a correct collider or centerline.
    #[func]
    fn paint_frame(&self) -> Transform3D {
        match self.world_shape() {
            render::Shape::Box3d { center, basis, .. } => Transform3D::new(
                Basis::from_cols(
                    Vector3::new(basis[0][0] as f32, basis[0][1] as f32, basis[0][2] as f32),
                    Vector3::new(basis[1][0] as f32, basis[1][1] as f32, basis[1][2] as f32),
                    Vector3::new(basis[2][0] as f32, basis[2][1] as f32, basis[2][2] as f32),
                ),
                Vector3::new(center[0] as f32, center[1] as f32, center[2] as f32),
            ),
            // WaveWall currently has one box shape. Keep this ClassDB test
            // observer total if the broader render vocabulary grows before
            // the observer is updated.
            _ => Transform3D::IDENTITY,
        }
    }

    /// One canonical box frame shared by the mesh/collider observer and the
    /// analytic superface boundary: exact snapped basis, with the floor datum
    /// lifted to the box center. Keeping this concrete avoids recovering a
    /// transform by destructuring the broader, extensible `render::Shape`
    /// enum at a ClassDB boundary.
    fn geometry_frame(&self) -> Transform3D {
        let placed = self.canonical_transform();
        let lift = Vector3::new(0.0, (level_plan::WALL_H * 0.5) as f32, 0.0);
        Transform3D::new(placed.basis, placed * lift)
    }

    /// The wall-owned shape witness for WaveLevel's editor condition watch.
    /// Other solids keep `WaveSkin` as a direct child; WaveWall deliberately
    /// nests it under the exact top-level physics body, so the generic direct-
    /// child lookup cannot see a length-only edit. Returning the mesh-local
    /// AABB here preserves that watch without recursively absorbing unrelated
    /// censused descendants.
    pub(crate) fn signature_aabb(&self) -> Option<[f32; 6]> {
        let aabb = self.mesh.as_ref()?.get_aabb();
        Some([
            aabb.position.x,
            aabb.position.y,
            aabb.position.z,
            aabb.size.x,
            aabb.size.y,
            aabb.size.z,
        ])
    }

    /// The engine-facing read-back of
    /// [`INode3D::get_configuration_warnings`] — needed for the same
    /// reason [`super::level::WaveLevel`]'s own forwarder carries one: that
    /// override is a pure GDVIRTUAL Godot's editor calls directly through
    /// the C++ virtual table and never binds to `ClassDB`, so no script can
    /// reach it under that name. Same disambiguation as [`Self::oid`] above
    /// — an inherent `#[func]` of the same name, forwarded through UFCS so
    /// it calls the trait override instead of recursing into itself.
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        INode3D::get_configuration_warnings(self)
    }

    /// This wall's centerline as the classic (x1, z1, x2, z2) segment —
    /// the level root derives every contract from these. Tree-only, and
    /// read WHOLE from one global transform: the same placement that puts
    /// the box in front of the eye decides which axis the run goes down,
    /// so the occluder can never end up perpendicular to the wall it
    /// describes.
    pub(crate) fn segment(&self) -> Vector4 {
        let placed = self.canonical_transform();
        let quadrant = level_plan::basis_quadrant(placed.basis);
        level_plan::wall_segment(placed.origin, self.length, quadrant)
    }

    /// Stage this wall for a level derivation. An explicit `rederive()` may
    /// happen in the same callback that changed an ancestor, before any editor
    /// process frame. WaveLevel calls this narrow boundary first so paint,
    /// placement and occlusion never depend on ambient scene-tree ordering.
    pub(crate) fn prepare_for_derive(&mut self) {
        self.snap_to_axis();
    }

    /// The axis law, enforced in WORLD space: whatever free-hand rotation
    /// (or scale) reaches this node — its own, or inherited from any
    /// ancestor above it — collapses onto the nearest exact quarter turn.
    /// World space is the whole point: the centerline is derived there, so
    /// snapping the LOCAL basis would leave a wall in a turned room drawing
    /// down one axis and occluding down the other. A quadrant basis has
    /// unit columns, so writing it globally discards inherited scale with
    /// the same stroke — `length` stays the one size knob however deep a
    /// room prefab nests the wall. Loud when it actually moved something —
    /// the designer should learn the law, not fight ghosts.
    fn snap_to_axis(&mut self) {
        if self.normalizing_transform {
            return;
        }
        let parent = if self.base().is_set_as_top_level() {
            None
        } else {
            self.base()
                .get_parent_node_3d()
                .map(|parent| parent.get_global_transform())
        };
        let local = self.base().get_transform();
        let current = self.base().get_global_transform();
        let plan = level_plan::plan_wall_transform(current, local, parent, self.transform_memory);
        self.transform_memory = plan.memory;
        let warning = match plan.memory.fault {
            Some(level_plan::WallTransformFault::Ancestor) => Some(ANCESTOR_TRANSFORM_WARNING),
            Some(level_plan::WallTransformFault::Own) => Some(OWN_TRANSFORM_WARNING),
            None => None,
        };
        self.set_transform_warning(warning);
        if plan.announce_snap && !Engine::singleton().is_editor_hint() {
            godot_warn!(
                "WaveWall '{}': snapped to the nearest quarter turn — walls are \
                 axis-aligned boxes by law (use the length knob, never scale)",
                self.base().get_name(),
            );
        }
        if let Some(placed) = plan.write_global {
            self.write_global_transform(placed, parent);
        }
        self.sync_body_transform();
    }

    /// The one guarded global write. Its exact finite result becomes the
    /// recovery point before entering Godot. On the next frame the global
    /// placement is already snapped (within any parent round-trip), so the
    /// adapter returns without another write and preserves any recovery warning
    /// until a later valid authored move actually needs normalization.
    fn write_global_transform(&mut self, placed: Transform3D, parent: Option<Transform3D>) {
        self.normalization_writes = self.normalization_writes.saturating_add(1);
        self.normalizing_transform = true;
        self.base_mut().set_global_transform(placed);
        self.normalizing_transform = false;
        self.transform_memory = level_plan::settle_wall_write(
            self.transform_memory,
            self.base().get_transform(),
            parent,
        );
    }

    /// The exact world transform every geometry consumer shares. A successful
    /// pure plan stores the desired quadrant transform before Godot maps it
    /// through the authored parent; an invalid episode keeps that last valid
    /// placement. Before the first valid plan, total recovery supplies a
    /// finite identity-quadrant fallback from whatever lanes the engine can
    /// still expose.
    fn canonical_transform(&self) -> Transform3D {
        self.transform_memory.last_finite.unwrap_or_else(|| {
            level_plan::recover_wall_transform(self.base().get_global_transform(), None)
        })
    }

    /// Move the private generated StaticBody—not its CollisionShape child—to
    /// the canonical world transform. PhysicsServer composes a shape's local
    /// transform with its owning body, so this is the only hierarchy that
    /// makes the visual mesh, collider, painted faces and occluder consume the
    /// same exact pose under an arbitrary authoring parent.
    fn sync_body_transform(&mut self) {
        let placed = self.canonical_transform();
        if let Some(body) = self.body.as_mut()
            && body.get_global_transform() != placed
        {
            self.body_transform_writes = self.body_transform_writes.saturating_add(1);
            body.set_global_transform(placed);
        }
    }

    /// Apply WaveWall's explicit, narrow collision contract to its private
    /// strikeable body. WaveWall is a Node3D datum rather than a dummy
    /// StaticBody proxy, so no inherited shape-owner/axis-lock/exception state
    /// is silently promised; each exported property setter calls this directly.
    fn sync_body_contract(&mut self) {
        let layer = self.collision_layer;
        let mask = self.collision_mask;
        let priority = self.collision_priority as f32;
        let ray_pickable = self.ray_pickable;
        let capture_input_on_drag = self.input_capture_on_drag;
        let physics_material = self.physics_material_override.clone();
        let (priority, _) =
            level_plan::sanitize_wall_priority(priority, self.priority_memory.last_valid);
        if let Some(body) = self.body.as_mut() {
            if body.get_collision_layer() != layer {
                body.set_collision_layer(layer);
                self.body_contract_writes = self.body_contract_writes.saturating_add(1);
            }
            if body.get_collision_mask() != mask {
                body.set_collision_mask(mask);
                self.body_contract_writes = self.body_contract_writes.saturating_add(1);
            }
            if body.get_collision_priority() != priority {
                body.set_collision_priority(priority);
                self.body_contract_writes = self.body_contract_writes.saturating_add(1);
            }
            if body.is_ray_pickable() != ray_pickable {
                body.set_ray_pickable(ray_pickable);
                self.body_contract_writes = self.body_contract_writes.saturating_add(1);
            }
            if body.get_capture_input_on_drag() != capture_input_on_drag {
                body.set_capture_input_on_drag(capture_input_on_drag);
                self.body_contract_writes = self.body_contract_writes.saturating_add(1);
            }
            let same_material = match (
                body.get_physics_material_override(),
                physics_material.as_ref(),
            ) {
                (None, None) => true,
                (Some(actual), Some(expected)) => actual == *expected,
                _ => false,
            };
            if !same_material {
                body.set_physics_material_override(physics_material.as_ref());
                self.body_contract_writes = self.body_contract_writes.saturating_add(1);
            }
        }
    }

    /// Keep one stable invalid episode. Editor mode stores the repair on the
    /// warning triangle and stays out of the output panel; runtime says it
    /// once. Repeated ancestor notifications neither log nor schedule writes,
    /// and clearing the state refreshes the icon exactly once as well.
    fn set_transform_warning(&mut self, warning: Option<&str>) {
        if self.transform_warning.as_deref() == warning {
            return;
        }
        self.transform_warning = warning.map(str::to_owned);
        if let Some(warning) = warning
            && self.base().is_inside_tree()
            && !Engine::singleton().is_editor_hint()
        {
            let detail = warning.strip_prefix("WaveWall: ").unwrap_or(warning);
            godot_warn!("WaveWall '{}': {}", self.base().get_name(), detail);
        }
        self.base_mut()
            .call_deferred("update_configuration_warnings", &[]);
    }

    /// Physics-property poison is independent of transform recovery: both can
    /// be visible at once, and fixing either must clear only its own icon line.
    fn set_body_contract_warning(&mut self, warning: Option<&str>) {
        if self.body_contract_warning.as_deref() == warning {
            return;
        }
        self.body_contract_warning = warning.map(str::to_owned);
        if let Some(warning) = warning
            && self.base().is_inside_tree()
            && !Engine::singleton().is_editor_hint()
        {
            let detail = warning.strip_prefix("WaveWall: ").unwrap_or(warning);
            godot_warn!("WaveWall '{}': {}", self.base().get_name(), detail);
        }
        self.base_mut()
            .call_deferred("update_configuration_warnings", &[]);
    }

    fn set_length_warning(&mut self, warning: Option<&str>) {
        if self.length_warning.as_deref() == warning {
            return;
        }
        self.length_warning = warning.map(str::to_owned);
        if let Some(warning) = warning
            && self.base().is_inside_tree()
            && !Engine::singleton().is_editor_hint()
        {
            let detail = warning.strip_prefix("WaveWall: ").unwrap_or(warning);
            godot_warn!("WaveWall '{}': {}", self.base().get_name(), detail);
        }
        self.base_mut()
            .call_deferred("update_configuration_warnings", &[]);
    }

    fn say_pending_length_warning(&self) {
        if let Some(warning) = self.length_warning.as_deref()
            && !Engine::singleton().is_editor_hint()
        {
            let detail = warning.strip_prefix("WaveWall: ").unwrap_or(warning);
            godot_warn!("WaveWall '{}': {}", self.base().get_name(), detail);
        }
    }

    fn say_pending_body_contract_warning(&self) {
        if let Some(warning) = self.body_contract_warning.as_deref()
            && !Engine::singleton().is_editor_hint()
        {
            let detail = warning.strip_prefix("WaveWall: ").unwrap_or(warning);
            godot_warn!("WaveWall '{}': {}", self.base().get_name(), detail);
        }
    }
}

#[godot_dyn]
impl WaveSolid for WaveWall {
    fn set_material(&mut self, mat: &Gd<Material>) {
        self.skin.set_material(mat);
    }

    /// This wall's box as `render::Shape`, in world space — the geometry
    /// the derive-time paint pass folds into the superface graph. Mirrors
    /// exactly what `ready()` builds: [`level_plan::wall_box`], centered at
    /// the same lift the mesh is drawn at (`(0, WALL_H/2, 0)` local),
    /// carried into world space by the same stored canonical transform as
    /// the generated mesh and physics body. The authored container may retain
    /// a few low bits of parent round-trip dust; it is deliberately not a
    /// second geometry source.
    fn world_shape(&self) -> render::Shape {
        let frame = self.geometry_frame();
        let size = level_plan::wall_box(self.length);
        render::Shape::Box3d {
            center: solid::to_f64_3(frame.origin),
            size: solid::to_f64_3(size),
            basis: solid::basis_columns_f64(frame.basis),
        }
    }

    /// Bake the derive-time paint pass's labels onto this wall — see
    /// [`solid::paint_solid`].
    fn paint(&mut self, labels_by_ordinal: &[f32]) {
        solid::paint_solid(
            self.mesh.as_mut(),
            render::paint::ShapeKind::Box,
            labels_by_ordinal,
        );
    }
}
