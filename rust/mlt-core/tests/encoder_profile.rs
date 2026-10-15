use mlt_core::encoder::{EncoderConfig, Network, Profile, Tessellation, WireVersion};

#[test]
fn the_default_profile_is_native() {
    assert_eq!(EncoderConfig::default().profile(), Profile::Native);
}

#[test]
fn the_default_network_is_4g() {
    assert_eq!(EncoderConfig::default().network(), Network::Cellular4G);
}

#[test]
fn the_default_tessellation_is_auto() {
    assert_eq!(EncoderConfig::default().tessellation(), Tessellation::Auto);
}

#[test]
fn the_native_profile_tessellates_automatically() {
    let native = EncoderConfig::default().with_profile(Profile::Native);
    assert_eq!(native.tessellation(), Tessellation::Auto);
}

#[test]
fn the_native_profile_allows_fastpfor() {
    let native = EncoderConfig::default().with_profile(Profile::Native);
    assert!(native.allow_fastpfor());
}

#[test]
fn the_native_profile_allows_fsst() {
    let native = EncoderConfig::default().with_profile(Profile::Native);
    assert!(native.allow_fsst());
}

#[test]
fn the_native_profile_allows_shared_dictionaries() {
    let native = EncoderConfig::default().with_profile(Profile::Native);
    assert!(native.allow_shared_dict());
}

#[test]
fn the_native_profile_keeps_the_source_order() {
    let native = EncoderConfig::default().with_profile(Profile::Native);
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
fn the_web_profile_stores_no_triangles() {
    let web = EncoderConfig::default().with_profile(Profile::Web);
    assert_eq!(web.tessellation(), Tessellation::Off);
}

#[test]
fn the_web_profile_switches_fastpfor_off() {
    let web = EncoderConfig::default().with_profile(Profile::Web);
    assert!(!web.allow_fastpfor());
}

#[test]
fn the_web_profile_switches_fsst_off() {
    let web = EncoderConfig::default().with_profile(Profile::Web);
    assert!(!web.allow_fsst());
}

#[test]
fn the_web_profile_keeps_shared_dictionaries() {
    let web = EncoderConfig::default().with_profile(Profile::Web);
    assert!(web.allow_shared_dict());
}

#[test]
fn the_web_profile_keeps_fastpfor_offline() {
    let web_offline = EncoderConfig::default()
        .with_profile(Profile::Web)
        .with_network(Network::Offline);
    assert!(web_offline.allow_fastpfor());
}

#[test]
fn the_web_profile_keeps_fsst_offline() {
    let web_offline = EncoderConfig::default()
        .with_profile(Profile::Web)
        .with_network(Network::Offline);
    assert!(web_offline.allow_fsst());
}

#[test]
fn the_web_profile_switches_fastpfor_off_when_the_network_is_set_first() {
    let web_on_5g = EncoderConfig::default()
        .with_network(Network::Cellular5G)
        .with_profile(Profile::Web);
    assert!(!web_on_5g.allow_fastpfor());
}

#[test]
fn the_min_size_profile_stores_no_triangles() {
    let min_size = EncoderConfig::default().with_profile(Profile::MinSize);
    assert_eq!(min_size.tessellation(), Tessellation::Off);
}

#[test]
fn the_min_size_profile_tries_hilbert_and_id_sorts() {
    let min_size = EncoderConfig::default().with_profile(Profile::MinSize);
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
fn the_min_size_profile_allows_delta2_under_v2() {
    let min_size_v2 = EncoderConfig::default()
        .with_wire_version(WireVersion::V02)
        .with_profile(Profile::MinSize);
    assert!(min_size_v2.allow_delta2());
}

#[test]
fn the_min_size_profile_allows_rans_vertices_under_v2() {
    let min_size_v2 = EncoderConfig::default()
        .with_wire_version(WireVersion::V02)
        .with_profile(Profile::MinSize);
    assert!(min_size_v2.allow_rans_vertices());
}

#[test]
fn the_min_size_profile_allows_no_delta2_under_v1() {
    let min_size_v1 = EncoderConfig::default()
        .with_wire_version(WireVersion::V01)
        .with_profile(Profile::MinSize);
    assert!(!min_size_v1.allow_delta2());
}

#[test]
fn the_diffable_profile_stores_no_triangles() {
    let diffable = EncoderConfig::default().with_profile(Profile::Diffable);
    assert_eq!(diffable.tessellation(), Tessellation::Off);
}

#[test]
fn the_diffable_profile_switches_fsst_off() {
    let diffable = EncoderConfig::default().with_profile(Profile::Diffable);
    assert!(!diffable.allow_fsst());
}

#[test]
fn the_diffable_profile_switches_shared_dictionaries_off() {
    let diffable = EncoderConfig::default().with_profile(Profile::Diffable);
    assert!(!diffable.allow_shared_dict());
}

#[test]
fn the_diffable_profile_keeps_fastpfor() {
    let diffable = EncoderConfig::default().with_profile(Profile::Diffable);
    assert!(diffable.allow_fastpfor());
}

#[test]
fn the_diffable_profile_allows_delta2_under_v2() {
    let diffable_v2 = EncoderConfig::default()
        .with_wire_version(WireVersion::V02)
        .with_profile(Profile::Diffable);
    assert!(diffable_v2.allow_delta2());
}

#[test]
fn fsst_set_before_the_web_profile_stays_on() {
    let web_with_fsst = EncoderConfig::default()
        .with_fsst(true)
        .with_profile(Profile::Web);
    assert!(web_with_fsst.allow_fsst());
}

#[test]
fn fsst_set_after_the_web_profile_stays_on() {
    let web_with_fsst = EncoderConfig::default()
        .with_profile(Profile::Web)
        .with_fsst(true);
    assert!(web_with_fsst.allow_fsst());
}

#[test]
fn tessellation_set_before_the_native_profile_stays_off() {
    let native_without_triangles = EncoderConfig::default()
        .with_tessellation(Tessellation::Off)
        .with_profile(Profile::Native);
    assert_eq!(native_without_triangles.tessellation(), Tessellation::Off);
}

#[test]
fn tessellation_set_after_the_web_profile_stays_auto() {
    let web_with_triangles = EncoderConfig::default()
        .with_profile(Profile::Web)
        .with_tessellation(Tessellation::Auto);
    assert_eq!(web_with_triangles.tessellation(), Tessellation::Auto);
}
