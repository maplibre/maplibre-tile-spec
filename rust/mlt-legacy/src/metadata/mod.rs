#[expect(
    clippy::doc_markdown,
    clippy::struct_field_names,
    clippy::trivially_copy_pass_by_ref,
    rustdoc::invalid_html_tags
)]
pub mod proto_tileset;
pub mod stream;
pub mod stream_encoding;
pub mod tileset;
