//! The view allocates nothing (spec N1): once a layer is decoded, walking every
//! coordinate of every feature performs no heap allocation.
#![expect(
    unsafe_code,
    reason = "a counting global allocator has to implement GlobalAlloc"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use mlt_core::geo_traits::{
    CoordTrait, GeometryTrait, GeometryType as GeoKind, LineStringTrait, MultiLineStringTrait,
    MultiPointTrait, MultiPolygonTrait, PointTrait, PolygonTrait,
};
use mlt_core::test_helpers::{dec, parser};
use mlt_core::{LendingIterator, ParsedLayer, ParsedLayer01};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

fn allocs() -> usize {
    ALLOCS.load(Relaxed)
}

/// The counter is process-wide, so the tests of this binary must not overlap.
static SERIAL: Mutex<()> = Mutex::new(());

/// Sums every coordinate through the geo-traits surface alone.
fn walk(g: &impl GeometryTrait<T = i32>) -> i64 {
    fn coord(c: &impl CoordTrait<T = i32>) -> i64 {
        i64::from(c.x()) + i64::from(c.y()) + c.nth(2).map_or(0, i64::from)
    }
    fn line(l: &impl LineStringTrait<T = i32>) -> i64 {
        l.coords().map(|c| coord(&c)).sum()
    }
    fn polygon(p: &impl PolygonTrait<T = i32>) -> i64 {
        p.exterior().map_or(0, |e| line(&e)) + p.interiors().map(|i| line(&i)).sum::<i64>()
    }
    match g.as_type() {
        GeoKind::Point(p) => p.coord().map_or(0, |c| coord(&c)),
        GeoKind::LineString(l) => line(l),
        GeoKind::Polygon(p) => polygon(p),
        GeoKind::MultiPoint(mp) => mp
            .points()
            .map(|p| p.coord().map_or(0, |c| coord(&c)))
            .sum(),
        GeoKind::MultiLineString(mls) => mls.line_strings().map(|l| line(&l)).sum(),
        GeoKind::MultiPolygon(mpoly) => mpoly.polygons().map(|p| polygon(&p)).sum(),
        GeoKind::GeometryCollection(_)
        | GeoKind::Rect(_)
        | GeoKind::Triangle(_)
        | GeoKind::Line(_) => unreachable!("the view never produces these"),
    }
}

/// The first OMT fixture, by name.
fn fixture() -> Vec<u8> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test/expected/0x01/omt");
    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "mlt"))
        .collect();
    paths.sort();
    fs::read(&paths[0]).unwrap()
}

fn parsed_layers(bytes: &[u8]) -> Vec<ParsedLayer01<'_>> {
    let layers = parser().parse_layers(bytes).unwrap();
    dec()
        .decode_all(layers)
        .unwrap()
        .into_iter()
        .filter_map(|layer| match layer {
            ParsedLayer::Tag01(layer) => Some(layer),
            ParsedLayer::Tag02(layer) => Some(layer.into_layer()),
            ParsedLayer::Unknown(_) | _ => None,
        })
        .collect()
}

#[test]
fn walking_every_view_allocates_nothing() {
    let _serial = SERIAL.lock().unwrap();
    let bytes = fixture();
    let layers = parsed_layers(&bytes);
    let features: usize = layers.iter().map(ParsedLayer01::feature_count).sum();
    assert!(
        features > 100,
        "fixture too small to be meaningful: {features}"
    );

    let before = allocs();
    let mut sum = 0;
    let mut offenders: Vec<String> = Vec::new();
    for layer in &layers {
        let values = layer.geometry_values();
        for index in 0..values.feature_count() {
            let at = allocs();
            let view = values.geometry(index).unwrap();
            sum += walk(&view);
            sum += view.vertices().map(|v| i64::from(v.x)).sum::<i64>();
            let grew = allocs() - at;
            if grew > 0 && offenders.len() < 8 {
                // Diagnostics only run on the failure path, after the measurement of this feature.
                offenders.push(format!(
                    "{}[{index}] {:?}: +{grew} allocs, {} stored vertices",
                    layer.name(),
                    view.geometry_type(),
                    view.vertices().len()
                ));
            }
        }
    }
    assert_ne!(sum, 0);
    assert!(
        offenders.is_empty(),
        "the view path allocated for:\n{}",
        offenders.join("\n")
    );
    assert_eq!(
        allocs() - before,
        0,
        "the view path allocated outside features"
    );
}

#[test]
fn the_row_iterator_allocates_nothing_per_feature() {
    let _serial = SERIAL.lock().unwrap();
    let bytes = fixture();
    let layers = parsed_layers(&bytes);
    for layer in &layers {
        if layer.feature_count() < 2 {
            continue;
        }
        let mut features = layer.iter_features();
        // The iterator sizes its value buffer on the first step; after that nothing may grow.
        let first = features.next().unwrap().unwrap();
        let mut sum = walk(&first.geometry());
        let before = allocs();
        while let Some(feature) = features.next() {
            sum += walk(&feature.unwrap().geometry());
        }
        assert_eq!(
            allocs() - before,
            0,
            "layer {} allocated per feature",
            layer.name()
        );
        assert_ne!(sum, i64::MIN);
    }
}
