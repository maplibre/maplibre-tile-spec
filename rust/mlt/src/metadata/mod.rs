pub mod proto_tileset;
pub mod tileset;

#[derive(Debug, Clone, Copy)]
pub enum PhysicalLevelTechnique {
    None,
    FastPfor,
    Varint,
    Alp,
}
