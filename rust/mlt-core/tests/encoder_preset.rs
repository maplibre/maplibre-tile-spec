use mlt_core::encoder::{Client, EncoderConfig, Network, Profile, Tessellation, WireVersion};

#[test]
fn the_native_preset_tessellates_automatically_on_its_network() {
    let native_on_3g = EncoderConfig::from_profile(Profile::Native(Network::Cellular3G));
    assert_eq!(
        native_on_3g.tessellation(),
        Tessellation::Auto(Client::Native, Network::Cellular3G)
    );
}

#[test]
fn the_native_preset_allows_fastpfor() {
    let native = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G));
    assert!(native.allow_fastpfor());
}

#[test]
fn the_native_preset_allows_fsst() {
    let native = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G));
    assert!(native.allow_fsst());
}

#[test]
fn the_native_preset_allows_shared_dictionaries() {
    let native = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G));
    assert!(native.allow_shared_dict());
}

#[test]
fn the_native_preset_keeps_the_source_order() {
    let native = EncoderConfig::from_profile(Profile::Native(Network::Cellular4G));
    assert!(!native.attempt_spatial_morton_sort());
    assert!(!native.attempt_spatial_hilbert_sort());
    assert!(!native.attempt_id_sort());
}

#[test]
fn the_web_preset_stores_no_triangles() {
    let web = EncoderConfig::from_profile(Profile::Web(Network::Cellular4G));
    assert_eq!(web.tessellation(), Tessellation::Off);
}

#[test]
fn the_web_preset_switches_fastpfor_off() {
    let web = EncoderConfig::from_profile(Profile::Web(Network::Cellular4G));
    assert!(!web.allow_fastpfor());
}

#[test]
fn the_web_preset_switches_fsst_off() {
    let web = EncoderConfig::from_profile(Profile::Web(Network::Cellular4G));
    assert!(!web.allow_fsst());
}

#[test]
fn the_web_preset_keeps_shared_dictionaries() {
    let web = EncoderConfig::from_profile(Profile::Web(Network::Cellular4G));
    assert!(web.allow_shared_dict());
}

#[test]
fn the_web_preset_keeps_fastpfor_offline() {
    let web_offline = EncoderConfig::from_profile(Profile::Web(Network::Offline));
    assert!(web_offline.allow_fastpfor());
}

#[test]
fn the_web_preset_keeps_fsst_offline() {
    let web_offline = EncoderConfig::from_profile(Profile::Web(Network::Offline));
    assert!(web_offline.allow_fsst());
}

#[test]
fn the_min_size_preset_stores_no_triangles() {
    let min_size = EncoderConfig::from_profile(Profile::MinSize);
    assert_eq!(min_size.tessellation(), Tessellation::Off);
}

#[test]
fn the_min_size_preset_tries_hilbert_and_id_sorts() {
    let min_size = EncoderConfig::from_profile(Profile::MinSize);
    assert!(!min_size.attempt_spatial_morton_sort());
    assert!(min_size.attempt_spatial_hilbert_sort());
    assert!(min_size.attempt_id_sort());
}

#[test]
fn the_min_size_preset_allows_delta2_under_v2() {
    let min_size_v2 =
        EncoderConfig::from_profile(Profile::MinSize).with_wire_version(WireVersion::V02);
    assert!(min_size_v2.allow_delta2());
}

#[test]
fn the_min_size_preset_allows_rans_vertices_under_v2() {
    let min_size_v2 =
        EncoderConfig::from_profile(Profile::MinSize).with_wire_version(WireVersion::V02);
    assert!(min_size_v2.allow_rans_vertices());
}

#[test]
fn the_min_size_preset_allows_no_delta2_under_v1() {
    let min_size_v1 =
        EncoderConfig::from_profile(Profile::MinSize).with_wire_version(WireVersion::V01);
    assert!(!min_size_v1.allow_delta2());
}

#[test]
fn the_diffable_preset_stores_no_triangles() {
    let diffable = EncoderConfig::from_profile(Profile::Diffable);
    assert_eq!(diffable.tessellation(), Tessellation::Off);
}

#[test]
fn the_diffable_preset_switches_fsst_off() {
    let diffable = EncoderConfig::from_profile(Profile::Diffable);
    assert!(!diffable.allow_fsst());
}

#[test]
fn the_diffable_preset_switches_shared_dictionaries_off() {
    let diffable = EncoderConfig::from_profile(Profile::Diffable);
    assert!(!diffable.allow_shared_dict());
}

#[test]
fn the_diffable_preset_keeps_fastpfor() {
    let diffable = EncoderConfig::from_profile(Profile::Diffable);
    assert!(diffable.allow_fastpfor());
}

#[test]
fn the_diffable_preset_allows_delta2_under_v2() {
    let diffable_v2 =
        EncoderConfig::from_profile(Profile::Diffable).with_wire_version(WireVersion::V02);
    assert!(diffable_v2.allow_delta2());
}

#[test]
fn fsst_set_on_the_web_preset_stays_on() {
    let web_with_fsst =
        EncoderConfig::from_profile(Profile::Web(Network::Cellular4G)).with_fsst(true);
    assert!(web_with_fsst.allow_fsst());
}

#[test]
fn tessellation_set_on_the_web_preset_stays_auto() {
    let web_with_triangles = EncoderConfig::from_profile(Profile::Web(Network::Cellular4G))
        .with_tessellation(Tessellation::Auto(Client::GlJs, Network::Cellular4G));
    assert_eq!(
        web_with_triangles.tessellation(),
        Tessellation::Auto(Client::GlJs, Network::Cellular4G)
    );
}

#[test]
fn the_fast_encoding_preset_stores_no_triangles() {
    let fast = EncoderConfig::from_profile(Profile::FastEncoding);
    assert_eq!(fast.tessellation(), Tessellation::Off);
}

#[test]
fn the_fast_encoding_preset_keeps_the_source_order() {
    let fast = EncoderConfig::from_profile(Profile::FastEncoding);
    assert!(!fast.attempt_spatial_morton_sort());
    assert!(!fast.attempt_spatial_hilbert_sort());
    assert!(!fast.attempt_id_sort());
}

#[test]
fn the_fast_encoding_preset_switches_fsst_off() {
    let fast = EncoderConfig::from_profile(Profile::FastEncoding);
    assert!(!fast.allow_fsst());
}

#[test]
fn the_fast_encoding_preset_switches_fastpfor_off() {
    let fast = EncoderConfig::from_profile(Profile::FastEncoding);
    assert!(!fast.allow_fastpfor());
}

#[test]
fn the_fast_encoding_preset_switches_shared_dictionaries_off() {
    let fast = EncoderConfig::from_profile(Profile::FastEncoding);
    assert!(!fast.allow_shared_dict());
}

#[test]
fn the_fast_encoding_preset_allows_no_delta2_under_v2() {
    let fast_v2 =
        EncoderConfig::from_profile(Profile::FastEncoding).with_wire_version(WireVersion::V02);
    assert!(!fast_v2.allow_delta2());
}

#[test]
fn the_fast_encoding_preset_allows_no_rans_vertices_under_v2() {
    let fast_v2 =
        EncoderConfig::from_profile(Profile::FastEncoding).with_wire_version(WireVersion::V02);
    assert!(!fast_v2.allow_rans_vertices());
}
