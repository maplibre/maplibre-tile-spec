//! Which of a city object's polygons each zoom's mesh holds.

use std::collections::HashSet;

use rustc_hash::{FxHashMap, FxHashSet};

use super::model::{Lods, OwnedPolygon, Part, SourcePolygon};

/// The polygons of the highest LOD of each owner and of its LOD2, each marked with the meshes it is in.
/// An owner whose polygons all lie in semantic surfaces picks these LODs for its walls, roofs and grounds each.
/// The LOD2 of an owner above LOD2 is its highest LOD of 2 or less, or else its highest LOD without windows, doors, `installations` and the holes they leave.
pub fn resolve(
    mut polygons: Vec<OwnedPolygon>,
    installations: &HashSet<u32>,
) -> Vec<SourcePolygon> {
    let mut owner_highest: FxHashMap<u32, u8> = FxHashMap::default();
    for p in &polygons {
        let h = owner_highest.entry(p.owner).or_insert(p.lod);
        *h = (*h).max(p.lod);
    }
    // The owners whose highest LOD has a polygon without semantics.
    let mut unsplit = FxHashSet::default();
    for p in &polygons {
        if p.polygon.part.is_none() && owner_highest.get(&p.owner) == Some(&p.lod) {
            unsplit.insert(p.owner);
        }
    }
    let key = |owner: u32, part: Option<Part>| {
        let kind = part.filter(|_| !unsplit.contains(&owner)).map(|p| match p {
            Part::Wall | Part::Window | Part::Door => Part::Wall,
            Part::Roof | Part::Ground => p,
        });
        (owner, kind)
    };
    polygons.retain(|p| p.polygon.part.is_some() || unsplit.contains(&p.owner));
    // The highest LOD of each key, and its highest LOD of 2 or less.
    let mut best: FxHashMap<(u32, Option<Part>), (u8, Option<u8>)> = FxHashMap::default();
    for p in &polygons {
        let (highest, low) = best
            .entry(key(p.owner, p.polygon.part))
            .or_insert((p.lod, None));
        *highest = (*highest).max(p.lod);
        if p.lod <= 2 {
            *low = (*low).max(Some(p.lod));
        }
    }
    let mut openings: FxHashMap<u32, Vec<Bounds3>> = FxHashMap::default();
    for p in &polygons {
        if matches!(p.polygon.part, Some(Part::Window | Part::Door))
            && let Some(bounds) = ring_bounds(&p.polygon.points)
        {
            openings.entry(p.owner).or_default().push(bounds);
        }
    }
    let mut resolved = Vec::with_capacity(polygons.len());
    for OwnedPolygon {
        owner,
        lod,
        polygon,
    } in polygons
    {
        let Some(&(highest, low)) = best.get(&key(owner, polygon.part)) else {
            continue;
        };
        let lods = if lod != highest {
            if Some(lod) != low {
                continue;
            }
            Lods::Lod2
        } else if highest <= 2 {
            Lods::Both
        } else if low.is_some()
            || installations.contains(&owner)
            || matches!(polygon.part, Some(Part::Window | Part::Door))
        {
            Lods::Highest
        } else if polygon.ends.len() > 1 {
            let closed =
                close_opening_holes(&polygon, openings.get(&owner).map_or(&[], Vec::as_slice));
            if let Some(lod2_polygon) = closed {
                resolved.push(lod2_polygon);
                Lods::Highest
            } else {
                Lods::Both
            }
        } else {
            Lods::Both
        };
        resolved.push(SourcePolygon { lods, ..polygon });
    }
    resolved
}

type Bounds3 = ([f64; 3], [f64; 3]);

fn ring_bounds(points: &[[f64; 3]]) -> Option<Bounds3> {
    let &first = points.first()?;
    Some(points[1..].iter().fold((first, first), |(lo, hi), &p| {
        (
            [0, 1, 2].map(|i| lo[i].min(p[i])),
            [0, 1, 2].map(|i| hi[i].max(p[i])),
        )
    }))
}

/// The LOD2 copy of `polygon`: a wall without its holes, or a roof or ground without those of window and door openings.
/// `None` if a roof or ground has no such hole.
fn close_opening_holes(
    polygon: &SourcePolygon,
    owner_openings: &[Bounds3],
) -> Option<SourcePolygon> {
    let exterior = polygon.ends[0];
    if polygon.part == Some(Part::Wall) {
        return Some(SourcePolygon {
            part: polygon.part,
            lods: Lods::Lod2,
            points: polygon.points[..exterior].to_vec(),
            ends: vec![exterior],
        });
    }
    if owner_openings.is_empty() {
        return None;
    }
    let mut points = polygon.points[..exterior].to_vec();
    let mut ends = vec![exterior];
    let mut dropped_any = false;
    let mut start = exterior;
    for &end in &polygon.ends[1..] {
        let hole = &polygon.points[start..end];
        let is_opening = ring_bounds(hole).is_some_and(|(r_lo, r_hi)| {
            let tol = [0, 1, 2].map(|i| 0.5 * (r_hi[i] - r_lo[i]).max(1e-6));
            owner_openings.iter().any(|&(o_lo, o_hi)| {
                (0..3).all(|i| {
                    (o_lo[i] - r_lo[i]).abs() <= tol[i] && (o_hi[i] - r_hi[i]).abs() <= tol[i]
                })
            })
        });
        if is_opening {
            dropped_any = true;
        } else {
            points.extend_from_slice(hole);
            ends.push(points.len());
        }
        start = end;
    }
    dropped_any.then_some(SourcePolygon {
        part: polygon.part,
        lods: Lods::Lod2,
        points,
        ends,
    })
}
