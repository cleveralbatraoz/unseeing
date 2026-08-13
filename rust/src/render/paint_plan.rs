//! Pure, atomic derivation of the labels a level boundary later applies.

use crate::oid_palette::Box3;
use crate::render::faces::{Face, Shape, faces};
use crate::render::labels;
use crate::render::paint::{self, ShapeKind, SourceRoleInput};
use crate::render::superface::{Superfaces, superfaces};

const LABEL_MIN: f64 = 0.15;
const LABEL_MAX: f64 = 0.96;

pub struct PaintEntryInput {
    pub area: Box3,
    pub shape: Shape,
    pub kind: ShapeKind,
    pub anchor: Option<f64>,
    pub is_wall: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintSourceInput {
    pub area: Option<Box3>,
    pub sweep_margin: f64,
    pub roles: u8,
}

pub struct PaintRequest {
    pub entries: Vec<PaintEntryInput>,
    pub sources: Vec<PaintSourceInput>,
    pub palette: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PaintCommand {
    KeepExisting,
    Relabel(Vec<f64>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintedFace {
    pub entry: usize,
    pub face: Face,
    pub label: f64,
    pub class: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryFault {
    InvalidArea,
    WrongFaceCount { actual: usize, expected: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexedEntryFault {
    pub entry: usize,
    pub fault: EntryFault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFault {
    InvalidArea,
    InvalidSweepMargin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexedSourceFault {
    pub source: usize,
    pub fault: SourceFault,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaintPlan {
    pub entry_commands: Vec<PaintCommand>,
    pub source_commands: Vec<PaintCommand>,
    pub faces: Vec<PaintedFace>,
    pub entry_faults: Vec<IndexedEntryFault>,
    pub source_faults: Vec<IndexedSourceFault>,
    pub starved_classes: Vec<usize>,
    pub starved_entries: Vec<usize>,
    pub starved_sources: Vec<usize>,
    pub wall_merge_entries: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintPlanError {
    EmptyPalette,
    InvalidPaletteValue {
        slot: usize,
    },
    PaletteConflict {
        first: usize,
        second: usize,
    },
    InvalidAnchor {
        entry: usize,
    },
    AnchorConflict {
        class: usize,
        first_entry: usize,
        second_entry: usize,
    },
    ClassOverflow,
    InvalidOutputLabel {
        class: usize,
    },
}

fn valid_label(label: f64) -> bool {
    label.is_finite() && (LABEL_MIN..=LABEL_MAX).contains(&label)
}

fn valid_box(area: Box3) -> bool {
    (0..3).all(|axis| {
        area.min[axis].is_finite() && area.max[axis].is_finite() && area.min[axis] <= area.max[axis]
    })
}

fn validate_palette(palette: &[f64]) -> Result<(), PaintPlanError> {
    if palette.is_empty() {
        return Err(PaintPlanError::EmptyPalette);
    }
    for (slot, &label) in palette.iter().enumerate() {
        if !valid_label(label) {
            return Err(PaintPlanError::InvalidPaletteValue { slot });
        }
    }
    for first in 0..palette.len() {
        for second in (first + 1)..palette.len() {
            if !labels::separated(palette[first], palette[second]) {
                return Err(PaintPlanError::PaletteConflict { first, second });
            }
        }
    }
    Ok(())
}

pub fn plan(request: PaintRequest) -> Result<PaintPlan, PaintPlanError> {
    validate_palette(&request.palette)?;
    for (entry, input) in request.entries.iter().enumerate() {
        if input.anchor.is_some_and(|label| !valid_label(label)) {
            return Err(PaintPlanError::InvalidAnchor { entry });
        }
    }

    let mut entry_commands = vec![PaintCommand::KeepExisting; request.entries.len()];
    let mut entry_faults = Vec::new();
    let mut accepted = vec![false; request.entries.len()];
    let face_capacity = request
        .entries
        .iter()
        .try_fold(0usize, |count, input| {
            count.checked_add(paint::face_count(input.kind))
        })
        .ok_or(PaintPlanError::ClassOverflow)?;
    let mut all_faces = Vec::with_capacity(face_capacity);
    let mut ordinal_of_face = Vec::with_capacity(face_capacity);
    for (entry, input) in request.entries.iter().enumerate() {
        if !valid_box(input.area) {
            entry_faults.push(IndexedEntryFault {
                entry,
                fault: EntryFault::InvalidArea,
            });
            continue;
        }
        let built = faces(entry, &input.shape);
        let expected = match input.kind {
            ShapeKind::Column => paint::face_count(input.kind) - 1,
            _ => paint::face_count(input.kind),
        };
        if built.len() != expected {
            entry_faults.push(IndexedEntryFault {
                entry,
                fault: EntryFault::WrongFaceCount {
                    actual: built.len(),
                    expected,
                },
            });
            continue;
        }
        accepted[entry] = true;
        ordinal_of_face.extend(0..built.len());
        all_faces.extend(built);
    }

    let pair_capacity = request
        .entries
        .len()
        .checked_mul(request.entries.len().saturating_sub(1))
        .and_then(|pairs| pairs.checked_div(2))
        .ok_or(PaintPlanError::ClassOverflow)?;
    let mut touching = Vec::with_capacity(pair_capacity);
    for first in 0..request.entries.len() {
        if !accepted[first] {
            continue;
        }
        for (second, second_accepted) in accepted.iter().enumerate().skip(first + 1) {
            if *second_accepted
                && request.entries[first]
                    .area
                    .touches(&request.entries[second].area)
            {
                touching.push((first, second));
            }
        }
    }
    let sf = superfaces(&all_faces, &touching);
    let flank_entries: Vec<usize> = request
        .entries
        .iter()
        .enumerate()
        .filter_map(|(entry, input)| {
            (accepted[entry] && input.kind == ShapeKind::Column).then_some(entry)
        })
        .collect();
    sf.classes
        .checked_add(flank_entries.len())
        .ok_or(PaintPlanError::ClassOverflow)?;
    let (flank_classes, classes, separations) =
        paint::add_flank_classes(&sf, &all_faces, &touching, &flank_entries);
    let mut flank_of_entry = vec![None; request.entries.len()];
    for (entry, class) in flank_entries.iter().copied().zip(flank_classes) {
        flank_of_entry[entry] = Some(class);
    }
    let mut classes_of_entry = vec![Vec::new(); request.entries.len()];
    for (face_index, face) in all_faces.iter().enumerate() {
        let Some(&class) = sf.class_of.get(face_index) else {
            return Err(PaintPlanError::ClassOverflow);
        };
        classes_of_entry[face.solid].push(class);
    }
    for (entry, class) in flank_of_entry.iter().copied().enumerate() {
        if let Some(class) = class {
            classes_of_entry[entry].push(class);
        }
    }

    let mut anchors: Vec<(usize, f64, usize)> = Vec::new();
    for (entry, input) in request.entries.iter().enumerate() {
        let Some(label) = input.anchor else { continue };
        let mut seen = Vec::new();
        for &class in &classes_of_entry[entry] {
            if seen.contains(&class) {
                continue;
            }
            seen.push(class);
            if let Some(&(_, prior, first_entry)) = anchors.iter().find(|&&(c, _, _)| c == class) {
                if prior.to_bits() != label.to_bits() {
                    return Err(PaintPlanError::AnchorConflict {
                        class,
                        first_entry,
                        second_entry: entry,
                    });
                }
            } else {
                anchors.push((class, label, entry));
            }
        }
    }
    let direct_anchors: Vec<(usize, f64)> = anchors
        .iter()
        .map(|&(class, label, _)| (class, label))
        .collect();

    let mut source_commands = vec![PaintCommand::KeepExisting; request.sources.len()];
    let mut source_faults = Vec::new();
    let mut valid_source_indices = Vec::new();
    let mut source_inputs = Vec::new();
    for (source, input) in request.sources.iter().enumerate() {
        let Some(area) = input.area else { continue };
        if !input.sweep_margin.is_finite() {
            source_faults.push(IndexedSourceFault {
                source,
                fault: SourceFault::InvalidSweepMargin,
            });
            continue;
        }
        if !valid_box(area) {
            source_faults.push(IndexedSourceFault {
                source,
                fault: SourceFault::InvalidArea,
            });
            continue;
        }
        valid_source_indices.push(source);
        source_inputs.push(SourceRoleInput {
            area: area.grown_flat(input.sweep_margin),
            roles: input.roles,
        });
    }
    let entry_areas: Vec<Box3> = request.entries.iter().map(|input| input.area).collect();
    let source_graph = paint::add_source_role_classes(
        classes,
        &separations,
        &source_inputs,
        &entry_areas,
        &classes_of_entry,
    );
    let expected_classes = source_inputs
        .iter()
        .try_fold(classes, |count, source| {
            count.checked_add(usize::from(source.roles))
        })
        .ok_or(PaintPlanError::ClassOverflow)?;
    if source_graph.classes != expected_classes {
        return Err(PaintPlanError::ClassOverflow);
    }
    let augmented = Superfaces {
        class_of: sf.class_of.clone(),
        classes: source_graph.classes,
        separations: source_graph.separations,
        cluster_of_solid: sf.cluster_of_solid.clone(),
    };
    let labelling = labels::assign(&augmented, &direct_anchors, &request.palette);
    for (class, &label) in labelling.label_of_class.iter().enumerate() {
        if !valid_label(label) {
            return Err(PaintPlanError::InvalidOutputLabel { class });
        }
    }

    for (entry, input) in request.entries.iter().enumerate() {
        if !accepted[entry] {
            continue;
        }
        let mut labels_by_ordinal = vec![0.0; paint::face_count(input.kind)];
        for (face_index, face) in all_faces.iter().enumerate() {
            if face.solid != entry {
                continue;
            }
            let Some(&ordinal) = ordinal_of_face.get(face_index) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            let Some(&class) = sf.class_of.get(face_index) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            let Some(&label) = labelling.label_of_class.get(class) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            let Some(slot) = labels_by_ordinal.get_mut(ordinal) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            *slot = label;
        }
        if let Some(class) = flank_of_entry[entry] {
            let Some(&label) = labelling.label_of_class.get(class) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            let Some(slot) = labels_by_ordinal.get_mut(2) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            *slot = label;
        }
        entry_commands[entry] = PaintCommand::Relabel(labels_by_ordinal);
    }
    for (source, role_classes) in valid_source_indices
        .iter()
        .copied()
        .zip(&source_graph.classes_of_source)
    {
        let labels = role_classes
            .iter()
            .map(|&class| {
                labelling
                    .label_of_class
                    .get(class)
                    .copied()
                    .ok_or(PaintPlanError::ClassOverflow)
            })
            .collect::<Result<Vec<_>, _>>()?;
        source_commands[source] = PaintCommand::Relabel(labels);
    }

    let faces = all_faces
        .iter()
        .enumerate()
        .map(|(face_index, face)| {
            let class = sf.class_of[face_index];
            PaintedFace {
                entry: face.solid,
                face: face.clone(),
                label: labelling.label_of_class[class],
                class,
            }
        })
        .collect();
    let starved_entries =
        paint::starved_entry_indices(&labelling.starved_classes, &classes_of_entry);
    let starved_sources = valid_source_indices
        .iter()
        .copied()
        .zip(&source_graph.classes_of_source)
        .filter_map(|(source, classes)| {
            classes
                .iter()
                .any(|class| labelling.starved_classes.contains(class))
                .then_some(source)
        })
        .collect();
    let wall_clusters: Vec<usize> = request
        .entries
        .iter()
        .enumerate()
        .filter_map(|(entry, input)| {
            (accepted[entry] && input.is_wall)
                .then(|| sf.cluster_of_solid.get(entry).copied())
                .flatten()
        })
        .collect();
    let wall_merge_entries = request
        .entries
        .iter()
        .enumerate()
        .filter_map(|(entry, input)| {
            (!input.is_wall
                && accepted[entry]
                && sf
                    .cluster_of_solid
                    .get(entry)
                    .is_some_and(|cluster| wall_clusters.contains(cluster)))
            .then_some(entry)
        })
        .collect();

    Ok(PaintPlan {
        entry_commands,
        source_commands,
        faces,
        entry_faults,
        source_faults,
        starved_classes: labelling.starved_classes,
        starved_entries,
        starved_sources,
        wall_merge_entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid_palette::Box3;
    use crate::render::faces::Shape;

    const IDENTITY: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    const PALETTE: [f64; 5] = [0.25, 0.34, 0.43, 0.52, 0.61];

    fn box_entry(center: [f64; 3], size: [f64; 3]) -> PaintEntryInput {
        PaintEntryInput {
            area: Box3::from_center_size(center, size),
            shape: Shape::Box3d {
                center,
                size,
                basis: IDENTITY,
            },
            kind: ShapeKind::Box,
            anchor: None,
            is_wall: false,
        }
    }

    fn request(entries: Vec<PaintEntryInput>) -> PaintRequest {
        PaintRequest {
            entries,
            sources: Vec::new(),
            palette: PALETTE.to_vec(),
        }
    }

    #[test]
    fn empty_palette_is_a_fatal_atomic_error() {
        let mut input = request(vec![box_entry([0.0; 3], [1.0; 3])]);
        input.palette.clear();
        assert_eq!(plan(input), Err(PaintPlanError::EmptyPalette));
    }

    #[test]
    fn palette_rejects_non_finite_out_of_band_and_too_close_values() {
        for (palette, expected) in [
            (
                vec![f64::NAN],
                PaintPlanError::InvalidPaletteValue { slot: 0 },
            ),
            (vec![0.05], PaintPlanError::InvalidPaletteValue { slot: 0 }),
            (
                vec![0.15, 0.229],
                PaintPlanError::PaletteConflict {
                    first: 0,
                    second: 1,
                },
            ),
        ] {
            let mut input = request(Vec::new());
            input.palette = palette;
            assert_eq!(plan(input), Err(expected));
        }
        assert!(
            plan(PaintRequest {
                entries: Vec::new(),
                sources: Vec::new(),
                palette: vec![0.15, 0.23, 0.96]
            })
            .is_ok()
        );
    }

    #[test]
    fn invalid_and_conflicting_anchors_are_fatal() {
        let mut invalid = box_entry([0.0; 3], [1.0; 3]);
        invalid.anchor = Some(0.05);
        assert_eq!(
            plan(request(vec![invalid])),
            Err(PaintPlanError::InvalidAnchor { entry: 0 })
        );

        let mut a = box_entry([0.0; 3], [2.0; 3]);
        a.anchor = Some(0.15);
        let mut b = box_entry([0.0; 3], [2.0; 3]);
        b.anchor = Some(0.90);
        assert_eq!(
            plan(request(vec![a, b])),
            Err(PaintPlanError::AnchorConflict {
                class: 0,
                first_entry: 0,
                second_entry: 1
            })
        );
    }

    #[test]
    fn malformed_entry_keeps_its_original_index_and_does_not_shift_later_faces() {
        let bad = box_entry([0.0; 3], [0.0; 3]);
        let good = box_entry([5.0, 0.0, 0.0], [1.0; 3]);
        let out = plan(request(vec![bad, good])).unwrap();
        assert_eq!(out.entry_commands[0], PaintCommand::KeepExisting);
        assert!(matches!(
            out.entry_faults.as_slice(),
            [IndexedEntryFault {
                entry: 0,
                fault: EntryFault::WrongFaceCount { .. }
            }]
        ));
        assert!(
            out.faces
                .iter()
                .all(|face| face.entry == 1 && face.face.solid == 1)
        );
        assert!(matches!(out.entry_commands[1], PaintCommand::Relabel(_)));
    }

    #[test]
    fn a_valid_box_returns_six_labelled_ordinals_and_face_records() {
        let out = plan(request(vec![box_entry([0.0; 3], [1.0; 3])])).unwrap();
        let PaintCommand::Relabel(labels) = &out.entry_commands[0] else {
            panic!("box was not painted")
        };
        assert_eq!(labels.len(), 6);
        assert!(labels.iter().all(|&label| label == 0.25));
        assert_eq!(out.faces.len(), 6);
        assert!(
            out.faces
                .iter()
                .all(|face| face.entry == 0 && face.label == 0.25)
        );
    }

    #[test]
    fn a_singleton_column_uses_one_label_for_both_rims_and_flank() {
        let entry = PaintEntryInput {
            area: Box3::from_center_size([0.0, 0.5, 0.0], [0.6, 1.0, 0.6]),
            shape: Shape::Column {
                center: [0.0, 0.5, 0.0],
                radius: 0.3,
                half_height: 0.5,
            },
            kind: ShapeKind::Column,
            anchor: None,
            is_wall: false,
        };
        let out = plan(request(vec![entry])).unwrap();
        assert_eq!(
            out.entry_commands,
            vec![PaintCommand::Relabel(vec![0.25; 3])]
        );
    }

    #[test]
    fn malformed_source_margin_is_indexed_and_later_source_roles_do_not_shift() {
        let source_area = Box3::from_center_size([0.0; 3], [1.0; 3]);
        let out = plan(PaintRequest {
            entries: Vec::new(),
            sources: vec![
                PaintSourceInput {
                    area: Some(source_area),
                    sweep_margin: f64::INFINITY,
                    roles: 2,
                },
                PaintSourceInput {
                    area: None,
                    sweep_margin: 0.0,
                    roles: 2,
                },
                PaintSourceInput {
                    area: Some(source_area),
                    sweep_margin: -1.0,
                    roles: 2,
                },
            ],
            palette: PALETTE.to_vec(),
        })
        .unwrap();
        assert_eq!(out.source_commands[0], PaintCommand::KeepExisting);
        assert_eq!(out.source_commands[1], PaintCommand::KeepExisting);
        assert!(
            matches!(out.source_commands[2], PaintCommand::Relabel(ref labels) if labels == &vec![0.25, 0.34])
        );
        assert_eq!(
            out.source_faults,
            vec![IndexedSourceFault {
                source: 0,
                fault: SourceFault::InvalidSweepMargin
            }]
        );
    }

    #[test]
    fn touching_sources_starve_by_original_source_owner() {
        let area = Box3::from_center_size([0.0; 3], [1.0; 3]);
        let out = plan(PaintRequest {
            entries: Vec::new(),
            sources: vec![
                PaintSourceInput {
                    area: Some(area),
                    sweep_margin: 0.0,
                    roles: 2,
                },
                PaintSourceInput {
                    area: None,
                    sweep_margin: 0.0,
                    roles: 2,
                },
                PaintSourceInput {
                    area: Some(area),
                    sweep_margin: 0.0,
                    roles: 2,
                },
            ],
            palette: vec![0.25, 0.34, 0.43],
        })
        .unwrap();
        assert_eq!(out.starved_sources, vec![2]);
        assert!(
            matches!(out.source_commands[2], PaintCommand::Relabel(ref labels) if labels.len() == 2)
        );
    }

    #[test]
    fn merged_non_wall_reports_its_original_entry_index() {
        let mut wall = box_entry([0.0; 3], [2.0; 3]);
        wall.is_wall = true;
        let separate = box_entry([8.0, 0.0, 0.0], [1.0; 3]);
        let merged = box_entry([0.0; 3], [2.0; 3]);
        let out = plan(request(vec![wall, separate, merged])).unwrap();
        assert_eq!(out.wall_merge_entries, vec![2]);
    }
}
