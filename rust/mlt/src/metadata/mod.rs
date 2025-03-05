pub mod proto_tileset;

#[derive(Debug, Clone, Copy)]
pub enum PhysicalLevelTechnique {
   None,
   FastPfor,
   Varint,
   Alp,
}
