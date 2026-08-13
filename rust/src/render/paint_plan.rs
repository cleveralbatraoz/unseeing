//! Pure, atomic derivation of the labels a level boundary later applies.

use std::collections::BTreeSet;

use crate::oid_palette::Box3;
use crate::render::faces::{Face, Shape, bounds, faces, paint_ordinal_count, planar_face_count};
use crate::render::labels;
use crate::render::superface::{Superfaces, superfaces};

const LABEL_MIN: f64 = 0.15;
const LABEL_MAX: f64 = 0.96;
pub const MAX_PAINT_ENTRIES: usize = 256;
pub const MAX_PAINT_SOURCES: usize = 256;
pub const MAX_PALETTE_VALUES: usize = 11;
pub const MAX_SOURCE_ROLES: usize = 512;
type Separations = Vec<(usize, usize)>;

struct SeparationBuilder {
    ordered: Separations,
    seen: BTreeSet<(usize, usize)>,
}

impl SeparationBuilder {
    fn from_existing(existing: &[(usize, usize)]) -> Self {
        let mut builder = Self {
            ordered: Vec::with_capacity(existing.len()),
            seen: BTreeSet::new(),
        };
        for &(a, b) in existing {
            builder.add(a, b);
        }
        builder
    }

    fn add(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let pair = if a < b { (a, b) } else { (b, a) };
        if self.seen.insert(pair) {
            self.ordered.push(pair);
        }
    }

    fn finish(self) -> Separations {
        self.ordered
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SourceRoleInput {
    pub(crate) area: Box3,
    pub(crate) roles: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceRoleGraph {
    pub(crate) classes: usize,
    pub(crate) separations: Separations,
    pub(crate) classes_of_source: Vec<Vec<usize>>,
}

fn separate_from_touching_neighbours(
    separations: &mut SeparationBuilder,
    class: usize,
    solid: usize,
    faces: &[Face],
    sf: &Superfaces,
    touching: &[(usize, usize)],
) {
    for &(a, b) in touching {
        let other = if a == solid {
            Some(b)
        } else if b == solid {
            Some(a)
        } else {
            None
        };
        let Some(other) = other else { continue };
        for (index, face) in faces.iter().enumerate() {
            if face.solid == other
                && let Some(&other_class) = sf.class_of.get(index)
            {
                separations.add(class, other_class);
            }
        }
    }
}

pub(crate) fn add_flank_classes(
    sf: &Superfaces,
    faces: &[Face],
    touching: &[(usize, usize)],
    flank_solids: &[usize],
) -> Result<(Vec<usize>, usize, Separations), PaintPlanError> {
    let mut classes = sf.classes;
    let mut separations = SeparationBuilder::from_existing(&sf.separations);
    let mut flank_classes = reserved(flank_solids.len());
    let cluster_span = sf
        .cluster_of_solid
        .iter()
        .copied()
        .max()
        .map(|max| max.checked_add(1).ok_or(PaintPlanError::ClassOverflow))
        .transpose()?
        .unwrap_or(0);
    let mut cluster_sizes = filled(cluster_span, 0usize);
    for &cluster in &sf.cluster_of_solid {
        let Some(size) = cluster_sizes.get_mut(cluster) else {
            return Err(PaintPlanError::ClassOverflow);
        };
        *size = size.checked_add(1).ok_or(PaintPlanError::ClassOverflow)?;
    }
    for &solid in flank_solids {
        let singleton = sf
            .cluster_of_solid
            .get(solid)
            .and_then(|cluster| cluster_sizes.get(*cluster))
            .is_some_and(|&size| size == 1);
        if singleton
            && let Some(class) = faces
                .iter()
                .position(|face| face.solid == solid)
                .and_then(|index| sf.class_of.get(index))
        {
            flank_classes.push(*class);
            continue;
        }
        let class = classes;
        classes = classes
            .checked_add(1)
            .ok_or(PaintPlanError::ClassOverflow)?;
        flank_classes.push(class);
        for (index, face) in faces.iter().enumerate() {
            if face.solid == solid {
                let Some(&rim_class) = sf.class_of.get(index) else {
                    return Err(PaintPlanError::ClassOverflow);
                };
                separations.add(class, rim_class);
            }
        }
        separate_from_touching_neighbours(&mut separations, class, solid, faces, sf, touching);
    }
    for (a_index, &a_solid) in flank_solids.iter().enumerate() {
        for (b_index, &b_solid) in flank_solids.iter().enumerate().skip(a_index + 1) {
            if touching
                .iter()
                .any(|&(x, y)| (x == a_solid && y == b_solid) || (x == b_solid && y == a_solid))
            {
                separations.add(flank_classes[a_index], flank_classes[b_index]);
            }
        }
    }
    Ok((flank_classes, classes, separations.finish()))
}

pub(crate) fn add_source_role_classes(
    classes: usize,
    separations: &[(usize, usize)],
    sources: &[SourceRoleInput],
    entry_areas: &[Box3],
    classes_of_entry: &[Vec<usize>],
) -> Result<SourceRoleGraph, PaintPlanError> {
    let world_classes = classes;
    let mut classes = classes;
    let mut separations = SeparationBuilder::from_existing(separations);
    let mut classes_of_source = reserved(sources.len());
    for source in sources {
        let role_count = usize::from(source.roles);
        let mut roles = reserved(role_count);
        for _ in 0..role_count {
            roles.push(classes);
            classes = classes
                .checked_add(1)
                .ok_or(PaintPlanError::ClassOverflow)?;
        }
        for (index, &a) in roles.iter().enumerate() {
            for &b in roles.iter().skip(index + 1) {
                separations.add(a, b);
            }
        }
        classes_of_source.push(roles);
    }
    for (source, roles) in sources.iter().zip(&classes_of_source) {
        for (entry_area, owned) in entry_areas.iter().zip(classes_of_entry) {
            if source.area.touches(entry_area) {
                for &role in roles {
                    for &class in owned {
                        if class < world_classes {
                            separations.add(role, class);
                        }
                    }
                }
            }
        }
    }
    for (a_index, (a, a_roles)) in sources.iter().zip(&classes_of_source).enumerate() {
        for (b, b_roles) in sources.iter().zip(&classes_of_source).skip(a_index + 1) {
            if a.area.touches(&b.area) {
                for &a_class in a_roles {
                    for &b_class in b_roles {
                        separations.add(a_class, b_class);
                    }
                }
            }
        }
    }
    Ok(SourceRoleGraph {
        classes,
        separations: separations.finish(),
        classes_of_source,
    })
}

pub(crate) fn starved_entry_indices(
    starved_classes: &[usize],
    classes_of_entry: &[Vec<usize>],
) -> Vec<usize> {
    classes_of_entry
        .iter()
        .enumerate()
        .filter_map(|(entry, classes)| {
            classes
                .iter()
                .any(|class| starved_classes.contains(class))
                .then_some(entry)
        })
        .collect()
}

/// Retain semantic-role assignments without shifting malformed positions.
#[must_use]
pub fn update_role_labels(previous: &[Option<f64>], supplied: &[f64]) -> Vec<Option<f64>> {
    let mut updated = previous.to_vec();
    updated.resize(updated.len().max(supplied.len()), None);
    for (role, &label) in supplied.iter().enumerate() {
        if label.is_finite() {
            updated[role] = Some(label);
        }
    }
    updated
}

pub struct PaintEntryInput {
    pub shape: Shape,
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
pub enum RequestDomain {
    Entries,
    Sources,
    PaletteValues,
    SourceRoles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintPlanError {
    RequestTooLarge {
        domain: RequestDomain,
        actual: usize,
        limit: usize,
    },
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
    AnchorSeparationConflict {
        first_class: usize,
        second_class: usize,
        first_entry: usize,
        second_entry: usize,
    },
    ClassOverflow,
    InvalidOutputLabel {
        class: usize,
    },
}

fn check_limit(domain: RequestDomain, actual: usize, limit: usize) -> Result<(), PaintPlanError> {
    if actual > limit {
        Err(PaintPlanError::RequestTooLarge {
            domain,
            actual,
            limit,
        })
    } else {
        Ok(())
    }
}

fn reserved<T>(capacity: usize) -> Vec<T> {
    Vec::with_capacity(capacity)
}

fn filled<T: Clone>(length: usize, value: T) -> Vec<T> {
    let mut values = reserved(length);
    values.resize(length, value);
    values
}

fn validate_request_size(request: &PaintRequest) -> Result<(), PaintPlanError> {
    check_limit(
        RequestDomain::Entries,
        request.entries.len(),
        MAX_PAINT_ENTRIES,
    )?;
    check_limit(
        RequestDomain::Sources,
        request.sources.len(),
        MAX_PAINT_SOURCES,
    )?;
    check_limit(
        RequestDomain::PaletteValues,
        request.palette.len(),
        MAX_PALETTE_VALUES,
    )?;
    let source_roles = request.sources.iter().try_fold(0usize, |count, source| {
        count.checked_add(usize::from(source.roles))
    });
    let Some(source_roles) = source_roles else {
        return Err(PaintPlanError::RequestTooLarge {
            domain: RequestDomain::SourceRoles,
            actual: usize::MAX,
            limit: MAX_SOURCE_ROLES,
        });
    };
    check_limit(RequestDomain::SourceRoles, source_roles, MAX_SOURCE_ROLES)?;
    Ok(())
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
    validate_request_size(&request)?;
    validate_palette(&request.palette)?;
    for (entry, input) in request.entries.iter().enumerate() {
        if input.anchor.is_some_and(|label| !valid_label(label)) {
            return Err(PaintPlanError::InvalidAnchor { entry });
        }
    }

    let mut entry_commands = filled(request.entries.len(), PaintCommand::KeepExisting);
    let mut entry_faults = reserved(request.entries.len());
    let mut accepted = filled(request.entries.len(), false);
    let mut entry_areas = filled(request.entries.len(), None);
    let face_capacity = request
        .entries
        .iter()
        .try_fold(0usize, |count, input| {
            count.checked_add(paint_ordinal_count(&input.shape))
        })
        .ok_or(PaintPlanError::ClassOverflow)?;
    let mut all_faces = reserved(face_capacity);
    let mut ordinal_of_face = reserved(face_capacity);
    for (entry, input) in request.entries.iter().enumerate() {
        let Some(area) = bounds(&input.shape) else {
            entry_faults.push(IndexedEntryFault {
                entry,
                fault: EntryFault::InvalidArea,
            });
            continue;
        };
        let built = faces(entry, &input.shape);
        let expected = planar_face_count(&input.shape);
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
        entry_areas[entry] = Some(area);
        ordinal_of_face.extend(0..built.len());
        all_faces.extend(built);
    }

    let pair_capacity = request
        .entries
        .len()
        .checked_mul(request.entries.len().saturating_sub(1))
        .and_then(|pairs| pairs.checked_div(2))
        .ok_or(PaintPlanError::ClassOverflow)?;
    let mut touching = reserved(pair_capacity);
    for first in 0..request.entries.len() {
        if !accepted[first] {
            continue;
        }
        for (second, second_accepted) in accepted.iter().enumerate().skip(first + 1) {
            if *second_accepted
                && entry_areas[first].is_some_and(|first_area| {
                    entry_areas[second].is_some_and(|second_area| first_area.touches(&second_area))
                })
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
            (accepted[entry] && matches!(input.shape, Shape::Column { .. })).then_some(entry)
        })
        .collect();
    sf.classes
        .checked_add(flank_entries.len())
        .ok_or(PaintPlanError::ClassOverflow)?;
    let (flank_classes, classes, separations) =
        add_flank_classes(&sf, &all_faces, &touching, &flank_entries)?;
    let mut flank_of_entry = filled(request.entries.len(), None);
    for (entry, class) in flank_entries.iter().copied().zip(flank_classes) {
        flank_of_entry[entry] = Some(class);
    }
    let mut classes_of_entry = filled(request.entries.len(), Vec::new());
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

    let mut anchor_by_class = filled(classes, None);
    for (entry, input) in request.entries.iter().enumerate() {
        let Some(label) = input.anchor else { continue };
        let mut seen = Vec::new();
        for &class in &classes_of_entry[entry] {
            if seen.contains(&class) {
                continue;
            }
            seen.push(class);
            let Some(slot) = anchor_by_class.get_mut(class) else {
                return Err(PaintPlanError::ClassOverflow);
            };
            if let Some((prior, first_entry)) = *slot {
                if f64::to_bits(prior) != label.to_bits() {
                    return Err(PaintPlanError::AnchorConflict {
                        class,
                        first_entry,
                        second_entry: entry,
                    });
                }
            } else {
                *slot = Some((label, entry));
            }
        }
    }
    let direct_anchors: Vec<(usize, f64)> = anchor_by_class
        .iter()
        .enumerate()
        .filter_map(|(class, anchor)| anchor.map(|(label, _)| (class, label)))
        .collect();
    for &(first_class, second_class) in &separations {
        let first = anchor_by_class.get(first_class).copied().flatten();
        let second = anchor_by_class.get(second_class).copied().flatten();
        if let (Some((first_label, first_entry)), Some((second_label, second_entry))) =
            (first, second)
            && !labels::separated(first_label, second_label)
        {
            return Err(PaintPlanError::AnchorSeparationConflict {
                first_class,
                second_class,
                first_entry,
                second_entry,
            });
        }
    }

    let mut source_commands = filled(request.sources.len(), PaintCommand::KeepExisting);
    let mut source_faults = reserved(request.sources.len());
    let mut valid_source_indices = reserved(request.sources.len());
    let mut source_inputs = reserved(request.sources.len());
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
        let swept_area = area.grown_flat(input.sweep_margin);
        if !valid_box(swept_area) {
            source_faults.push(IndexedSourceFault {
                source,
                fault: SourceFault::InvalidArea,
            });
            continue;
        }
        valid_source_indices.push(source);
        source_inputs.push(SourceRoleInput {
            area: swept_area,
            roles: input.roles,
        });
    }
    let accepted_entry_areas: Vec<Box3> = entry_areas.iter().flatten().copied().collect();
    let accepted_entry_classes: Vec<Vec<usize>> = classes_of_entry
        .iter()
        .zip(&accepted)
        .filter_map(|(classes, accepted)| accepted.then_some(classes.clone()))
        .collect();
    let source_graph = add_source_role_classes(
        classes,
        &separations,
        &source_inputs,
        &accepted_entry_areas,
        &accepted_entry_classes,
    )?;
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
        let mut labels_by_ordinal = vec![0.0; paint_ordinal_count(&input.shape)];
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
    let starved_entries = starved_entry_indices(&labelling.starved_classes, &classes_of_entry);
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
            shape: Shape::Box3d {
                center,
                size,
                basis: IDENTITY,
            },
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

    /// Palette validation is quadratic, so the public planner refuses an
    /// oversized request before comparing any pair or allocating graph state.
    #[test]
    fn oversized_palette_is_an_explicit_atomic_error() {
        let mut input = request(Vec::new());
        input.palette = vec![0.25; 12];
        assert_eq!(
            plan(input),
            Err(PaintPlanError::RequestTooLarge {
                domain: RequestDomain::PaletteValues,
                actual: 12,
                limit: 11,
            })
        );
    }

    #[test]
    fn two_hundred_fifty_seven_entries_are_an_explicit_atomic_error() {
        let entries = (0..257)
            .map(|index| box_entry([index as f64 * 10.0, 0.0, 0.0], [1.0; 3]))
            .collect();
        assert_eq!(
            plan(request(entries)),
            Err(PaintPlanError::RequestTooLarge {
                domain: RequestDomain::Entries,
                actual: 257,
                limit: 256,
            })
        );
    }

    #[test]
    fn two_hundred_fifty_seven_sources_are_an_explicit_atomic_error() {
        let input = PaintRequest {
            entries: Vec::new(),
            sources: vec![
                PaintSourceInput {
                    area: None,
                    sweep_margin: 0.0,
                    roles: 0,
                };
                257
            ],
            palette: PALETTE.to_vec(),
        };
        assert_eq!(
            plan(input),
            Err(PaintPlanError::RequestTooLarge {
                domain: RequestDomain::Sources,
                actual: 257,
                limit: 256,
            })
        );
    }

    #[test]
    fn five_hundred_thirteen_semantic_roles_are_rejected_even_when_sources_are_drawless() {
        let input = PaintRequest {
            entries: Vec::new(),
            sources: vec![
                PaintSourceInput {
                    area: None,
                    sweep_margin: 0.0,
                    roles: u8::MAX,
                },
                PaintSourceInput {
                    area: None,
                    sweep_margin: 0.0,
                    roles: u8::MAX,
                },
                PaintSourceInput {
                    area: None,
                    sweep_margin: 0.0,
                    roles: 3,
                },
            ],
            palette: PALETTE.to_vec(),
        };
        assert_eq!(
            plan(input),
            Err(PaintPlanError::RequestTooLarge {
                domain: RequestDomain::SourceRoles,
                actual: 513,
                limit: 512,
            })
        );
    }

    #[test]
    fn exactly_two_hundred_fifty_six_entries_and_their_faces_are_admitted() {
        let entries = (0..256)
            .map(|index| box_entry([index as f64 * 10.0, 0.0, 0.0], [1.0; 3]))
            .collect();
        let output = plan(request(entries)).unwrap();
        assert_eq!(output.entry_commands.len(), 256);
        assert_eq!(output.faces.len(), 1_536);
    }

    #[test]
    fn exactly_two_hundred_fifty_six_sources_are_admitted() {
        let sources = vec![
            PaintSourceInput {
                area: None,
                sweep_margin: 0.0,
                roles: 0,
            };
            256
        ];
        let output = plan(PaintRequest {
            entries: Vec::new(),
            sources,
            palette: PALETTE.to_vec(),
        })
        .unwrap();
        assert_eq!(output.source_commands.len(), 256);
    }

    #[test]
    fn exactly_eleven_separated_palette_values_are_admitted() {
        let output = plan(PaintRequest {
            entries: Vec::new(),
            sources: Vec::new(),
            palette: vec![
                0.15, 0.23, 0.31, 0.39, 0.47, 0.55, 0.63, 0.71, 0.79, 0.87, 0.95,
            ],
        })
        .unwrap();
        assert!(output.entry_commands.is_empty());
        assert!(output.source_commands.is_empty());
    }

    #[test]
    fn exactly_five_hundred_twelve_drawless_source_roles_are_admitted() {
        let sources = vec![
            PaintSourceInput {
                area: None,
                sweep_margin: 0.0,
                roles: u8::MAX,
            },
            PaintSourceInput {
                area: None,
                sweep_margin: 0.0,
                roles: u8::MAX,
            },
            PaintSourceInput {
                area: None,
                sweep_margin: 0.0,
                roles: 2,
            },
        ];
        let output = plan(PaintRequest {
            entries: Vec::new(),
            sources,
            palette: PALETTE.to_vec(),
        })
        .unwrap();
        assert_eq!(output.source_commands.len(), 3);
        assert!(
            output
                .source_commands
                .iter()
                .all(|command| *command == PaintCommand::KeepExisting)
        );
    }

    #[test]
    fn palette_rejects_non_finite_out_of_band_and_too_close_values() {
        for (palette, expected) in [
            (
                vec![f64::NAN],
                PaintPlanError::InvalidPaletteValue { slot: 0 },
            ),
            (vec![0.05], PaintPlanError::InvalidPaletteValue { slot: 0 }),
            (vec![0.961], PaintPlanError::InvalidPaletteValue { slot: 0 }),
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
    fn touching_separate_anchors_that_cannot_draw_a_seam_are_fatal() {
        let mut first = box_entry([0.0, 0.0, 0.0], [2.0, 1.0, 2.0]);
        first.anchor = Some(0.15);
        let mut second = box_entry([2.0, 0.0, 0.0], [2.0, 1.0, 2.0]);
        second.anchor = Some(0.20);
        assert_eq!(
            plan(request(vec![first, second])),
            Err(PaintPlanError::AnchorSeparationConflict {
                first_class: 0,
                second_class: 1,
                first_entry: 0,
                second_entry: 1,
            })
        );
    }

    /// Touching is derived from the shape itself, so two geometrically
    /// abutting solids cannot evade the fixed-label seam constraint.
    #[test]
    fn geometrically_abutting_shapes_enforce_their_seam() {
        let mut first = box_entry([0.0, 0.0, 0.0], [2.0, 1.0, 2.0]);
        first.anchor = Some(0.15);
        let mut second = box_entry([2.0, 0.0, 0.0], [2.0, 1.0, 2.0]);
        second.anchor = Some(0.20);

        assert!(matches!(
            plan(request(vec![first, second])),
            Err(PaintPlanError::AnchorSeparationConflict { .. })
        ));
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
    fn malformed_entry_areas_keep_existing_at_their_original_indices() {
        let good = box_entry([0.0; 3], [1.0; 3]);
        let mut reversed = box_entry([5.0, 0.0, 0.0], [1.0; 3]);
        reversed.shape = Shape::Column {
            center: [f64::INFINITY, 0.0, 0.0],
            radius: 1.0,
            half_height: 1.0,
        };
        let mut poisoned = box_entry([10.0, 0.0, 0.0], [1.0; 3]);
        poisoned.shape = Shape::Column {
            center: [0.0, f64::NAN, 0.0],
            radius: 1.0,
            half_height: 1.0,
        };
        let out = plan(request(vec![good, reversed, poisoned])).unwrap();
        assert!(matches!(out.entry_commands[0], PaintCommand::Relabel(_)));
        assert_eq!(out.entry_commands[1], PaintCommand::KeepExisting);
        assert_eq!(out.entry_commands[2], PaintCommand::KeepExisting);
        assert_eq!(
            out.entry_faults,
            vec![
                IndexedEntryFault {
                    entry: 1,
                    fault: EntryFault::InvalidArea
                },
                IndexedEntryFault {
                    entry: 2,
                    fault: EntryFault::InvalidArea
                },
            ]
        );
    }

    #[test]
    fn malformed_source_areas_keep_existing_at_their_original_indices() {
        let good = Box3::from_center_size([0.0; 3], [1.0; 3]);
        let reversed = Box3 {
            min: [2.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        };
        let mut poisoned = good;
        poisoned.max[1] = f64::INFINITY;
        let out = plan(PaintRequest {
            entries: Vec::new(),
            sources: vec![
                PaintSourceInput {
                    area: Some(good),
                    sweep_margin: 0.0,
                    roles: 1,
                },
                PaintSourceInput {
                    area: Some(reversed),
                    sweep_margin: 0.0,
                    roles: 1,
                },
                PaintSourceInput {
                    area: Some(poisoned),
                    sweep_margin: 0.0,
                    roles: 1,
                },
            ],
            palette: PALETTE.to_vec(),
        })
        .unwrap();
        assert!(matches!(out.source_commands[0], PaintCommand::Relabel(_)));
        assert_eq!(out.source_commands[1], PaintCommand::KeepExisting);
        assert_eq!(out.source_commands[2], PaintCommand::KeepExisting);
        assert_eq!(
            out.source_faults,
            vec![
                IndexedSourceFault {
                    source: 1,
                    fault: SourceFault::InvalidArea
                },
                IndexedSourceFault {
                    source: 2,
                    fault: SourceFault::InvalidArea
                },
            ]
        );
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
            shape: Shape::Column {
                center: [0.0, 0.5, 0.0],
                radius: 0.3,
                half_height: 0.5,
            },
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

    /// A finite margin can still overflow the post-growth sweep box. That
    /// malformed result must be rejected before touch tests see infinities.
    #[test]
    fn finite_source_margin_that_overflows_the_sweep_keeps_existing() {
        let out = plan(PaintRequest {
            entries: Vec::new(),
            sources: vec![PaintSourceInput {
                area: Some(Box3::from_center_size([f64::MAX, 0.0, 0.0], [1.0; 3])),
                sweep_margin: f64::MAX,
                roles: 1,
            }],
            palette: PALETTE.to_vec(),
        })
        .unwrap();

        assert_eq!(out.source_commands, vec![PaintCommand::KeepExisting]);
        assert_eq!(
            out.source_faults,
            vec![IndexedSourceFault {
                source: 0,
                fault: SourceFault::InvalidArea,
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
    mod graph_law_tests {
        use super::*;
        use crate::render::superface::superfaces;

        /// A merged class can be owned by several entries, and one entry can
        /// own several starved classes. Every affected owner is named once in
        /// entry order — never once per class and never only the first owner.
        #[test]
        fn starved_classes_recover_every_owner_once() {
            let classes_of_entry = vec![vec![1, 4], vec![4, 7], vec![8], vec![7, 9]];
            assert_eq!(
                starved_entry_indices(&[4, 7], &classes_of_entry),
                vec![0, 1, 3]
            );
        }

        /// No starvation means no owners, while an unknown class is simply not
        /// owned. Both cases are total and need no caller-side bounds guard.
        #[test]
        fn empty_or_unknown_starvation_recovers_no_entries() {
            let classes_of_entry = vec![vec![1, 4], vec![], vec![8]];
            assert!(starved_entry_indices(&[], &classes_of_entry).is_empty());
            assert!(starved_entry_indices(&[99], &classes_of_entry).is_empty());
        }

        // ---------------------------------------------------------------
        // add_flank_classes
        // ---------------------------------------------------------------

        fn separated(a: f64, b: f64) -> bool {
            (a - b).abs() >= labels::MIN_SEP - 1e-9
        }

        /// An empty `flank_solids` is a pure no-op: `sf`'s own class count and
        /// separations pass through untouched, so a level with no columns pays
        /// nothing for this pass.
        #[test]
        fn no_flanks_leaves_the_graph_unchanged() {
            let f = faces(
                0,
                &Shape::Column {
                    center: [0.0; 3],
                    radius: 0.3,
                    half_height: 0.5,
                },
            );
            let sf = superfaces(&f, &[]);
            let (flank_class, classes, seps) = add_flank_classes(&sf, &f, &[], &[]).unwrap();
            assert!(flank_class.is_empty());
            assert_eq!(classes, sf.classes);
            assert_eq!(seps, sf.separations);
        }

        /// Wave S: THE case the brief names, re-derived. A column standing
        /// flush on the floor is an ABUTMENT, not a merge — the column's
        /// bottom rim faces DOWN, the floor's top faces UP, opposite
        /// directions — so the column stays alone in its own cluster exactly
        /// as `superfaces`'s own singleton collapse defines one. The flank no
        /// longer wins a fresh class separated from its own rims: it JOINS
        /// the rim's already-collapsed class, and the whole column (rims and
        /// flank alike, now one class) still separates from the FLOOR's own
        /// class through rule (c)'s ordinary blanket law, since floor and
        /// column are different clusters and touching — end to end through
        /// `labels::assign`, exactly as `test_a_flank_separates_from_its_rims_and_a_touching_neighbour`
        /// (`game/tests/map_test.gd`) now checks on the real node → mesh
        /// pipeline.
        #[test]
        fn a_column_flush_on_the_floor_reads_as_one_uniform_class() {
            let floor = Shape::Box3d {
                center: [0.0, -0.05, 0.0],
                size: [10.0, 0.1, 10.0],
                basis: IDENTITY,
            };
            let column = Shape::Column {
                center: [0.0, 0.5, 0.0],
                radius: 0.3,
                half_height: 0.5,
            };
            let mut all = faces(0, &floor);
            all.extend(faces(1, &column));
            let touching = [(0, 1)];
            let sf = superfaces(&all, &touching);
            // the column's two rims (global 6, 7 — after the floor's six faces)
            assert_eq!(all[6].normal, [0.0, -1.0, 0.0]);
            assert_eq!(all[7].normal, [0.0, 1.0, 0.0]);
            // the abutment: never merged, so the rims collapsed to ONE class
            assert_eq!(sf.class_of[6], sf.class_of[7]);
            assert_ne!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);

            let (flank_class, classes, seps) =
                add_flank_classes(&sf, &all, &touching, &[1]).unwrap();
            // no new class allocated: the flank ALIASES the rims' own
            assert_eq!(classes, sf.classes);
            let flank = flank_class[0];
            assert_eq!(flank, sf.class_of[6]);
            assert_eq!(flank, sf.class_of[7]);
            assert_eq!(seps, sf.separations);

            let augmented = Superfaces {
                class_of: sf.class_of.clone(),
                classes,
                separations: seps,
                cluster_of_solid: sf.cluster_of_solid.clone(),
            };
            let out = labels::assign(&augmented, &[], &PALETTE);
            assert_eq!(out.starved, 0);
            let flank_label = out.label_of_class[flank];
            // no internal seam: rim and flank read the identical label
            assert_eq!(flank_label, out.label_of_class[sf.class_of[6]]);
            assert_eq!(flank_label, out.label_of_class[sf.class_of[7]]);
            // the outer seam still draws: the column (rims+flank, one class)
            // differs from the floor's own class
            assert!(separated(flank_label, out.label_of_class[sf.class_of[0]]));
        }

        /// Two columns standing flush against each other must not let their
        /// flanks share a label where the curves meet — the same law the old
        /// whole-box touch graph already held, and this campaign must not
        /// regress it.
        #[test]
        fn two_touching_columns_separate_their_flanks() {
            // Centers far enough apart that neither rim's circle overlaps the
            // other's (distance 5.0 >> 2 * radius 0.3) — `touching` is supplied
            // directly, as the real level's AABB touch walk would, so this
            // isolates the flank-vs-flank rule from any incidental rim merge.
            let a = Shape::Column {
                center: [0.0, 0.5, 0.0],
                radius: 0.3,
                half_height: 0.5,
            };
            let b = Shape::Column {
                center: [5.0, 0.5, 0.0],
                radius: 0.3,
                half_height: 0.5,
            };
            let mut all = faces(0, &a);
            all.extend(faces(1, &b));
            let touching = [(0, 1)];
            let sf = superfaces(&all, &touching);
            let (flank_class, _classes, seps) =
                add_flank_classes(&sf, &all, &touching, &[0, 1]).unwrap();
            assert!(seps.contains(&ordered(flank_class[0], flank_class[1])));
        }

        /// Wave S review finding (IMPORTANT): the MULTI-MEMBER branch (lines
        /// above, a column whose rim genuinely coplanar-MERGES with a
        /// partner rather than merely abutting one) was mutation-dead — every
        /// other flank fixture in this file ends up a SINGLETON, since a
        /// column resting ON anything presents an opposite-facing surface (a
        /// buried abutment, never a same-direction merge). This fixture
        /// forces a genuine merge instead: `post`'s TOP rim is flush with,
        /// and faces the SAME way as, `block`'s own top face, entirely
        /// inside its footprint — a coplanar overlap that MERGES them,
        /// putting `block` and `post` in the SAME multi-member cluster.
        /// `post`'s bottom rim stays clear (y=0, strictly inside `block`'s
        /// own y-range, matching none of its six faces), so exactly one
        /// merge edge exists — the minimal case that still makes the cluster
        /// multi-member.
        ///
        /// The two assertions are chosen to be independently diagnostic:
        /// `post`'s own bottom rim (global 6) is never one of `block`'s real
        /// faces, so that pair can ONLY come from the own-rim loop (476-480);
        /// `block`'s own -X face (global 0) is never one of `post`'s real
        /// faces, so that pair can ONLY come from the touching-neighbour loop
        /// (`separate_from_touching_neighbours`, formerly inlined at
        /// 482-496) — deleting either loop alone, or both together, must
        /// fail at least one.
        #[test]
        fn a_columns_flank_separates_from_a_genuinely_merged_rim_and_its_partner() {
            let block = Shape::Box3d {
                center: [0.0, 0.0, 0.0],
                size: [6.0, 2.0, 6.0],
                basis: IDENTITY,
            };
            let post = Shape::Column {
                center: [0.0, 0.5, 0.0],
                radius: 0.3,
                half_height: 0.5,
            };
            let mut all = faces(0, &block);
            all.extend(faces(1, &post));

            // the merge premise, hand-derived: post's top rim (global 7,
            // y=1) is flush with and faces the same way as block's own top
            // (global 3, y=1) — block spans y in [-1,1], post spans y in
            // [0,1], so post's bottom rim (global 6, y=0) matches neither
            // block's own -Y (y=-1) nor any other block face.
            assert_eq!(all[3].normal, [0.0, 1.0, 0.0]);
            assert_eq!(all[3].offset, 1.0);
            assert_eq!(all[7].normal, [0.0, 1.0, 0.0]);
            assert_eq!(all[7].offset, 1.0);
            assert_eq!(all[6].normal, [0.0, -1.0, 0.0]);
            assert_eq!(all[6].offset, 0.0);
            assert_eq!(all[0].normal, [-1.0, 0.0, 0.0]);

            let touching = [(0, 1)];
            let sf = superfaces(&all, &touching);
            // the merge actually happened: block and post share ONE
            // multi-member cluster now, neither is a singleton
            assert_eq!(sf.class_of[3], sf.class_of[7]);
            assert_eq!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);

            let (flank_class, classes, seps) =
                add_flank_classes(&sf, &all, &touching, &[1]).unwrap();
            let flank = flank_class[0];
            // own-rim loop: post's bottom rim, never one of block's faces
            assert!(seps.contains(&ordered(flank, sf.class_of[6])));
            // touching-neighbour loop: block's own -X, never one of post's
            assert!(seps.contains(&ordered(flank, sf.class_of[0])));

            let augmented = Superfaces {
                class_of: sf.class_of.clone(),
                classes,
                separations: seps,
                cluster_of_solid: sf.cluster_of_solid.clone(),
            };
            let out = labels::assign(&augmented, &[], &PALETTE);
            assert_eq!(out.starved, 0);
            let flank_label = out.label_of_class[flank];
            assert!(separated(flank_label, out.label_of_class[sf.class_of[6]]));
            assert!(separated(flank_label, out.label_of_class[sf.class_of[0]]));
        }

        /// Wave S review finding (MINOR 1): a `flank_solids` entry naming a
        /// solid with no real face in `faces` at all — never true of an
        /// actual column today (`column_faces` has no degeneracy guard of
        /// its own and always emits its two rims), but a defensive property
        /// of THIS function's own contract, not a currently-reachable path —
        /// used to fall back to a fresh, wholly UNCONSTRAINED class: free to
        /// land on whatever slot a touching neighbour already uses, a silent
        /// melt. `far` (solid 2) exists only to push `solid_count` past
        /// index 1, so solid 1 still gets a valid (trivial, singleton)
        /// cluster entry rather than an out-of-range one — the exact
        /// shape `is_singleton`'s `.get(solid)` needs to read `true` and
        /// reach the orphan-fallback arm at all.
        #[test]
        fn a_flank_naming_a_faceless_solid_still_separates_from_its_touching_neighbours() {
            let block = Shape::Box3d {
                center: [0.0, 0.0, 0.0],
                size: [1.0, 1.0, 1.0],
                basis: IDENTITY,
            };
            let far = Shape::Box3d {
                center: [50.0, 0.0, 0.0],
                size: [1.0, 1.0, 1.0],
                basis: IDENTITY,
            };
            let mut all = faces(0, &block);
            all.extend(faces(2, &far)); // solid 1 deliberately absent
            let touching = [(0, 1)];
            let sf = superfaces(&all, &touching);
            assert_eq!(sf.cluster_of_solid.len(), 3);
            // solid 1 got a real, trivial singleton cluster despite
            // contributing no face at all — the premise the fallback branch
            // needs to even be reached
            assert_ne!(sf.cluster_of_solid[0], sf.cluster_of_solid[1]);

            let (flank_class, classes, seps) =
                add_flank_classes(&sf, &all, &touching, &[1]).unwrap();
            let flank = flank_class[0];
            assert_eq!(classes, sf.classes + 1);
            // separated from EVERY class block's six real faces belong to —
            // the fix; the unfixed fallback added no separation at all
            for (i, face) in all.iter().enumerate() {
                if face.solid == 0 {
                    assert!(seps.contains(&ordered(flank, sf.class_of[i])));
                }
            }

            let augmented = Superfaces {
                class_of: sf.class_of.clone(),
                classes,
                separations: seps,
                cluster_of_solid: sf.cluster_of_solid.clone(),
            };
            let out = labels::assign(&augmented, &[], &PALETTE);
            assert_eq!(out.starved, 0);
            let flank_label = out.label_of_class[flank];
            for &c in &sf.class_of[0..6] {
                assert!(separated(flank_label, out.label_of_class[c]));
            }
        }

        /// Wave S: a column with no neighbours at all is alone in its own
        /// cluster — its two rims already collapsed to ONE class by
        /// `superfaces`'s own singleton law before this function ever runs —
        /// and the flank JOINS that same class rather than winning a fresh,
        /// separated one: today's look, a lone barrel with no internal seam
        /// at all. No class allocated, no separation added.
        #[test]
        fn an_isolated_columns_flank_joins_its_solids_singleton_class() {
            let f = faces(
                0,
                &Shape::Column {
                    center: [0.0; 3],
                    radius: 0.3,
                    half_height: 0.5,
                },
            );
            let sf = superfaces(&f, &[]);
            assert_eq!(sf.classes, 1);
            assert_eq!(sf.class_of[0], sf.class_of[1]);

            let (flank_class, classes, seps) = add_flank_classes(&sf, &f, &[], &[0]).unwrap();
            assert_eq!(classes, sf.classes);
            assert_eq!(seps, sf.separations);
            assert!(seps.is_empty());
            assert_eq!(flank_class[0], sf.class_of[0]);
            assert_eq!(flank_class[0], sf.class_of[1]);
        }

        fn ordered(a: usize, b: usize) -> (usize, usize) {
            if a < b { (a, b) } else { (b, a) }
        }

        /// Two same-class sources are still two authored objects. Give each of
        /// their two semantic roles a colourable class; because the source boxes
        /// touch, all four role classes separate from one another. The one world
        /// class touching both completes a hand-derived K5, exactly coverable by
        /// the five world labels with no seam starvation.
        #[test]
        fn touching_source_roles_join_the_same_separation_graph_as_world_faces() {
            let world = Box3::from_center_size([0.0, 0.5, 0.0], [2.0, 1.0, 1.0]);
            let sources = [
                SourceRoleInput {
                    area: Box3::from_center_size([0.6, 0.5, 0.0], [0.2, 1.0, 1.0]),
                    roles: 2,
                },
                SourceRoleInput {
                    area: Box3::from_center_size([0.8, 0.5, 0.0], [0.2, 1.0, 1.0]),
                    roles: 2,
                },
            ];
            let graph = add_source_role_classes(1, &[], &sources, &[world], &[vec![0]]).unwrap();
            assert_eq!(graph.classes, 5);
            assert_eq!(graph.classes_of_source, vec![vec![1, 2], vec![3, 4]]);
            for a in 0..5 {
                for b in (a + 1)..5 {
                    assert!(graph.separations.contains(&(a, b)), "missing {a}--{b}");
                }
            }
            let out = labels::assign(
                &Superfaces {
                    class_of: vec![0],
                    classes: graph.classes,
                    separations: graph.separations,
                    cluster_of_solid: vec![0],
                },
                &[],
                &PALETTE,
            );
            assert_eq!(out.starved, 0);
            for a in 0..5 {
                for b in (a + 1)..5 {
                    assert!(separated(out.label_of_class[a], out.label_of_class[b]));
                }
            }
        }

        /// The largest admitted role population can form one clique. Building
        /// it must remain practical and contain every pair exactly once; a
        /// linear scan for each insertion turns this case cubic.
        #[test]
        fn maximum_source_role_clique_has_one_edge_per_pair() {
            let sources = [
                SourceRoleInput {
                    area: Box3::from_center_size([0.0; 3], [1.0; 3]),
                    roles: u8::MAX,
                },
                SourceRoleInput {
                    area: Box3::from_center_size([0.0; 3], [1.0; 3]),
                    roles: u8::MAX,
                },
                SourceRoleInput {
                    area: Box3::from_center_size([0.0; 3], [1.0; 3]),
                    roles: 2,
                },
            ];

            let graph = add_source_role_classes(0, &[], &sources, &[], &[]).unwrap();
            assert_eq!(graph.classes, 512);
            assert_eq!(graph.separations.len(), 130_816);
            assert_eq!(graph.separations.first(), Some(&(0, 1)));
            assert_eq!(graph.separations.last(), Some(&(509, 511)));
        }

        /// Repeated ownership and repeated seed edges are common graph paths,
        /// but each normalized pair remains present exactly once and keeps the
        /// first insertion position.
        #[test]
        fn separation_builder_deduplicates_without_reordering() {
            let source = SourceRoleInput {
                area: Box3::from_center_size([0.0; 3], [1.0; 3]),
                roles: 1,
            };
            let graph = add_source_role_classes(
                2,
                &[(1, 0), (0, 1)],
                &[source],
                &[source.area],
                &[vec![0, 0, 1, 1]],
            )
            .unwrap();

            assert_eq!(graph.separations, vec![(0, 1), (0, 2), (1, 2)]);
        }

        /// Distance is permission to reuse labels. Two source boxes that do not
        /// touch have no cross-source edges; each source's own roles still differ.
        #[test]
        fn separated_sources_reuse_the_same_role_labels() {
            let sources = [
                SourceRoleInput {
                    area: Box3::from_center_size([0.0; 3], [0.2; 3]),
                    roles: 2,
                },
                SourceRoleInput {
                    area: Box3::from_center_size([10.0, 0.0, 0.0], [0.2; 3]),
                    roles: 2,
                },
            ];
            let graph = add_source_role_classes(0, &[], &sources, &[], &[]).unwrap();
            let out = labels::assign(
                &Superfaces {
                    class_of: vec![],
                    classes: graph.classes,
                    separations: graph.separations,
                    cluster_of_solid: vec![],
                },
                &[],
                &PALETTE,
            );
            assert_eq!(out.starved, 0);
            assert!(separated(out.label_of_class[0], out.label_of_class[1]));
            assert_eq!(out.label_of_class[0], out.label_of_class[2]);
            assert_eq!(out.label_of_class[1], out.label_of_class[3]);
        }

        /// Malformed parallel entry inputs are truncated to their common prefix;
        /// unknown class indices and a zero-role source are ignored, never indexed.
        #[test]
        fn source_role_graph_is_total_for_mismatched_and_unknown_inputs() {
            let source = SourceRoleInput {
                area: Box3::from_center_size([0.0; 3], [1.0; 3]),
                roles: 0,
            };
            let graph = add_source_role_classes(
                2,
                &[(0, 1)],
                &[source],
                &[source.area, source.area],
                &[vec![99]],
            )
            .unwrap();
            assert_eq!(graph.classes, 2);
            assert_eq!(graph.separations, vec![(0, 1)]);
            assert_eq!(graph.classes_of_source, vec![vec![]]);
        }

        /// Boundary corruption cannot compact role indices or erase omitted
        /// assignments: Case remains Case even if the incoming Case value is NaN.
        #[test]
        fn malformed_or_short_role_updates_preserve_semantic_indices() {
            let previous = vec![Some(0.25), Some(0.34), Some(0.43)];
            assert_eq!(
                update_role_labels(&previous, &[f64::NAN, 0.52]),
                vec![Some(0.25), Some(0.52), Some(0.43)]
            );
            assert_eq!(update_role_labels(&previous, &[]), previous);
            assert_eq!(
                update_role_labels(&[], &[0.61, f64::INFINITY, 0.43]),
                vec![Some(0.61), None, Some(0.43)]
            );
        }
    }
}
