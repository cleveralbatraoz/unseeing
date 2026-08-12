//! The authored hero datum. `WaveSpawn` deliberately draws nothing: its
//! Marker3D editor glyph is the whole blueprint and the running game sees only
//! the global pose selected by the owning level.

use godot::classes::{IMarker3D, Marker3D, Node};
use godot::prelude::*;

use super::solid::warnings_from_level;

#[derive(GodotClass)]
#[class(tool, init, base=Marker3D)]
pub struct WaveSpawn {
    base: Base<Marker3D>,
}

#[godot_api]
impl IMarker3D for WaveSpawn {
    fn get_configuration_warnings(&self) -> PackedStringArray {
        warnings_from_level(&self.base().clone().upcast::<Node>())
    }
}

#[godot_api]
impl WaveSpawn {
    #[func]
    fn get_configuration_warnings(&self) -> PackedStringArray {
        IMarker3D::get_configuration_warnings(self)
    }
}
