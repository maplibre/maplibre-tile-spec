
#[derive(Debug)]
pub struct GeometryColumn {
    pub geometry_types: Vec<i32>,
    pub num_geometries: Vec<i32>,
    pub num_parts: Vec<i32>,
    pub num_rings: Vec<i32>,
    pub vertex_offsets: Option<Vec<i32>>,
    pub vertex_list: Vec<i32>,
}
