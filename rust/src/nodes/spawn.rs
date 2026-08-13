//! The authored hero datum. `WaveSpawn` deliberately draws nothing: its
//! Marker3D editor glyph is the whole blueprint and the running game sees only
//! the global pose selected by the owning level.

use godot::classes::{IMarker3D, Marker3D, Node};
use godot::prelude::*;

use super::solid::warnings_from_level;

/// The drawless start marker for a WaveLevel.
///
/// Place one WaveSpawn under the level. Its global position and facing choose
/// where the hero wakes; the Marker3D glyph is its complete editor blueprint.
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

#[cfg(all(test, feature = "editor-docs"))]
mod tests {
    /// A drawless datum has no runtime mesh to explain itself, so its class
    /// description is part of the usable editor blueprint rather than
    /// optional library prose.
    #[test]
    fn wave_spawn_class_description_reaches_editor_docs() {
        let xml = godot::docs::gather_xml_docs()
            .find(|xml| xml.contains("<class name=\"WaveSpawn\""))
            .expect("WaveSpawn must register an editor-docs XML class");
        assert!(
            xml.contains("The drawless start marker for a WaveLevel"),
            "WaveSpawn editor XML has no class overview: {xml}"
        );
    }
}
