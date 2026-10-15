use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};

#[test]
fn the_default_is_the_native_preset_on_4g() {
    assert_eq!(
        EncoderConfig::default(),
        EncoderConfig::from_preset(Profile::Native, Network::Cellular4G)
    );
}

#[test]
fn the_default_tessellation_is_auto() {
    assert_eq!(EncoderConfig::default().tessellation(), Tessellation::Auto);
}

#[test]
fn a_preset_keeps_its_network() {
    let native_on_3g = EncoderConfig::from_preset(Profile::Native, Network::Cellular3G);
    assert_eq!(native_on_3g.network(), Network::Cellular3G);
}

#[test]
fn the_native_preset_tessellates_automatically() {
    let native = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G);
    assert_eq!(native.tessellation(), Tessellation::Auto);
}

#[test]
fn the_native_preset_allows_fastpfor() {
    let native = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G);
    assert!(native.allow_fastpfor());
}

#[test]
fn the_native_preset_allows_fsst() {
    let native = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G);
    assert!(native.allow_fsst());
}

#[test]
fn the_native_preset_allows_shared_dictionaries() {
    let native = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G);
    assert!(native.allow_shared_dict());
}

#[test]
fn the_native_preset_keeps_the_source_order() {
    let native = EncoderConfig::from_preset(Profile::Native, Network::Cellular4G);
    assert_eq!(
        (
            native.attempt_spatial_morton_sort(),
            native.attempt_spatial_hilbert_sort(),
            native.attempt_id_sort()
        ),
        (false, false, false)
    );
}

#[test]
fn the_web_preset_stores_no_triangles() {
    let web = EncoderConfig::from_preset(Profile::Web, Network::Cellular4G);
    assert_eq!(web.tessellation(), Tessellation::Off);
}

#[test]
fn the_web_preset_switches_fastpfor_off() {
    let web = EncoderConfig::from_preset(Profile::Web, Network::Cellular4G);
    assert!(!web.allow_fastpfor());
}

#[test]
fn the_web_preset_switches_fsst_off() {
    let web = EncoderConfig::from_preset(Profile::Web, Network::Cellular4G);
    assert!(!web.allow_fsst());
}

#[test]
fn the_web_preset_keeps_shared_dictionaries() {
    let web = EncoderConfig::from_preset(Profile::Web, Network::Cellular4G);
    assert!(web.allow_shared_dict());
}

#[test]
fn the_web_preset_keeps_fastpfor_offline() {
    let web_offline = EncoderConfig::from_preset(Profile::Web, Network::Offline);
    assert!(web_offline.allow_fastpfor());
}

#[test]
fn the_web_preset_keeps_fsst_offline() {
    let web_offline = EncoderConfig::from_preset(Profile::Web, Network::Offline);
    assert!(web_offline.allow_fsst());
}

#[test]
fn the_min_size_preset_stores_no_triangles() {
    let min_size = EncoderConfig::from_preset(Profile::MinSize, Network::Cellular4G);
    assert_eq!(min_size.tessellation(), Tessellation::Off);
}

#[test]
fn the_min_size_preset_tries_hilbert_and_id_sorts() {
    let min_size = EncoderConfig::from_preset(Profile::MinSize, Network::Cellular4G);
    assert_eq!(
        (
            min_size.attempt_spatial_morton_sort(),
            min_size.attempt_spatial_hilbert_sort(),
            min_size.attempt_id_sort()
        ),
        (false, true, true)
    );
}

#[test]
fn the_min_size_preset_allows_delta2_under_v2() {
    let min_size_v2 = EncoderConfig::from_preset(Profile::MinSize, Network::Cellular4G)
        .with_wire_version(WireVersion::V02);
    assert!(min_size_v2.allow_delta2());
}

#[test]
fn the_min_size_preset_allows_rans_vertices_under_v2() {
    let min_size_v2 = EncoderConfig::from_preset(Profile::MinSize, Network::Cellular4G)
        .with_wire_version(WireVersion::V02);
    assert!(min_size_v2.allow_rans_vertices());
}

#[test]
fn the_min_size_preset_allows_no_delta2_under_v1() {
    let min_size_v1 = EncoderConfig::from_preset(Profile::MinSize, Network::Cellular4G)
        .with_wire_version(WireVersion::V01);
    assert!(!min_size_v1.allow_delta2());
}

#[test]
fn the_diffable_preset_stores_no_triangles() {
    let diffable = EncoderConfig::from_preset(Profile::Diffable, Network::Cellular4G);
    assert_eq!(diffable.tessellation(), Tessellation::Off);
}

#[test]
fn the_diffable_preset_switches_fsst_off() {
    let diffable = EncoderConfig::from_preset(Profile::Diffable, Network::Cellular4G);
    assert!(!diffable.allow_fsst());
}

#[test]
fn the_diffable_preset_switches_shared_dictionaries_off() {
    let diffable = EncoderConfig::from_preset(Profile::Diffable, Network::Cellular4G);
    assert!(!diffable.allow_shared_dict());
}

#[test]
fn the_diffable_preset_keeps_fastpfor() {
    let diffable = EncoderConfig::from_preset(Profile::Diffable, Network::Cellular4G);
    assert!(diffable.allow_fastpfor());
}

#[test]
fn the_diffable_preset_allows_delta2_under_v2() {
    let diffable_v2 = EncoderConfig::from_preset(Profile::Diffable, Network::Cellular4G)
        .with_wire_version(WireVersion::V02);
    assert!(diffable_v2.allow_delta2());
}

#[test]
fn fsst_set_on_the_web_preset_stays_on() {
    let web_with_fsst =
        EncoderConfig::from_preset(Profile::Web, Network::Cellular4G).with_fsst(true);
    assert!(web_with_fsst.allow_fsst());
}

#[test]
fn tessellation_set_on_the_web_preset_stays_auto() {
    let web_with_triangles = EncoderConfig::from_preset(Profile::Web, Network::Cellular4G)
        .with_tessellation(Tessellation::Auto);
    assert_eq!(web_with_triangles.tessellation(), Tessellation::Auto);
}
