use std::iter::Map;
use std::sync::{Arc, Mutex};
use rusqlite::{Connection, Result, params, Row};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug)]
pub struct Tile {
    pub zoom_level: u32,
    pub tile_column: u32,
    pub tile_row: u32,
    pub tile_data: Vec<u8>,
}

struct MetadataRow {
    name: String,
    value: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct MbTilesMetadata{
    pub name: String,
    pub format: String,
    pub description: Option<String>,
    pub min_zoom: Option<u32>,
    pub max_zoom: Option<u32>,
    pub bounds: Option<String>,
    pub center: Option<String>,
    pub attribution: Option<String>,
    /* If the format is pbf, the metadata table MUST contain json (stringified JSON object):
     * lists the layers that appear in the vector tiles and the names and types of the attributes
     * of features that appear in those layers
    */
    pub json: Option<String>
}

pub struct MBTilesTileRepository {
    metadata: MbTilesMetadata,
    connection: Arc<Mutex<Connection>>
}

impl MBTilesTileRepository {

    pub fn new(connection: Connection) -> Self{
        let metadata = MBTilesTileRepository::query_metadata(&connection).unwrap();

        Self{
            metadata,
            connection: Arc::new(Mutex::new(connection))
        }
    }
    
    pub fn from_file(file_name: &str) -> Result<Self, ()> {
        let connection = match Connection::open(file_name){
            Ok(conn) => conn,
            Err(error) => panic!("Error while opening the MBTiles database {:?}", error)
        };

        Ok(MBTilesTileRepository::new(connection))
    }

    fn query_metadata(connection: &Connection) -> Result<MbTilesMetadata> {
        let mut stmt = connection.prepare(
            "SELECT name, value FROM metadata;",
        ).unwrap();

        let rows = stmt.query_map([], |row| {
            Ok(MetadataRow{
                name: row.get(0)?,
                value: row.get(1)?,
            })
        })?;

        let mut metadata = MbTilesMetadata{
            name: String::new(),
            format: String::new(),
            description: None,
            min_zoom: None,
            max_zoom: None,
            bounds: None,
            center: None,
            attribution: None,
            json: None
        };

        for result_metadata in rows {
            let m = &result_metadata?;
            let name = &m.name;
            let value = &m.value;

            match name.as_str() {
                "name" => metadata.name = value.to_string(),
                "format" => metadata.format = value.to_string(),
                "minzoom" => metadata.min_zoom = Some(value.parse::<u32>().unwrap()),
                "maxzoom" => metadata.max_zoom = Some(value.parse::<u32>().unwrap()),
                "attribution" => metadata.attribution = Some(value.to_string()),
                "json" => metadata.json = Some(value.to_string()),
                "description" => metadata.description = Some(value.to_string()),
                "bounds" => metadata.bounds = Some(value.to_string()),
                "center" => metadata.center = Some(value.to_string()),
                _ => {}
            }
        }

        return Ok(metadata);
    }

}

impl MBTilesTileRepository{
    pub fn get_metadata(&self) -> &MbTilesMetadata{
        &self.metadata
    }

    // pub fn get_tile(&self, x: u32, y: u32, z:u32) -> Option<Tile>{
    //     /* convert xyz to tms tiling scheme which is used in the MBTiles file */
    //     let tms_y = u32::pow(2, z) -y -1;
    //     let connection = self.connection.lock().unwrap();
    //     let mut stmt = connection.prepare(
    //         "SELECT zoom_level, tile_column, tile_row, tile_data FROM tiles WHERE zoom_level = ? AND tile_column = ? AND tile_row = ?;",
    //     ).unwrap();
    // 
    //     let mut tiles = stmt.query_map(params![ z.to_string().as_str(),x.to_string().as_str(), tms_y.to_string().as_str()], |row| {
    //         Ok(Tile {
    //             zoom_level: row.get(0)?,
    //             tile_column: row.get(1)?,
    //             tile_row: row.get(2)?,
    //             tile_data: row.get(3)?
    //         })
    //     }).unwrap();
    // 
    //     match tiles.next() {
    //         Some(t) => Some(t.unwrap()),
    //         None => None
    //     }
    // }
    pub fn get_tile(&self, col: u32, row: u32, zoom:u32) -> Option<Tile>{
        /* convert xyz to tms tiling scheme which is used in the MBTiles file */
        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT zoom_level, tile_column, tile_row, tile_data FROM tiles WHERE zoom_level = ? AND tile_column = ? AND tile_row = ?;",
        ).unwrap();

        let mut tiles = stmt.query_map(params![ zoom.to_string().as_str(), col.to_string().as_str(), row.to_string().as_str()], |row| {
            Ok(Tile {
                zoom_level: row.get(0)?,
                tile_column: row.get(1)?,
                tile_row: row.get(2)?,
                tile_data: row.get(3)?
            })
        }).unwrap();

        match tiles.next() {
            Some(t) => Some(t.unwrap()),
            None => None
        }
    }

    pub fn get_all_tiles(&self) -> Option<Vec<Tile>> {

        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT zoom_level, tile_column, tile_row, tile_data FROM tiles;",
        ).unwrap();

        let tiles = stmt.query_map(params![], |row| {
            Ok(Tile {
                zoom_level: row.get(0)?,
                tile_column: row.get(1)?,
                tile_row: row.get(2)?,
                tile_data: row.get(3)?
            })
        })
            .unwrap()
            .map(|x| x.unwrap())
            .collect::<Vec<Tile>>();

        Some(tiles)
    }

    pub fn map<F: Fn(&Tile)>(&self, f: F) {
        let connection = self.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT zoom_level, tile_column, tile_row, tile_data FROM tiles;",
        ).unwrap();

        print!("Iterating over mapdata");
        let _ = stmt.query_map(params![], |row| {
            print!(".");
            let t  = Tile {
                zoom_level: row.get(0)?,
                tile_column: row.get(1)?,
                tile_row: row.get(2)?,
                tile_data: row.get(3)?
            };
            f(&t);
            Ok(())
        })
            .unwrap();
        
        println!();
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{Connection};
    use crate::{tile_repository::MBTilesTileRepository};

    #[test]
    fn get_tile(){
        let conn = match Connection::open(":memory:"){
            Ok(conn) => conn,
            Err(error) => panic!("Error while opening the MBTiles database {:?}", error)
        };
        let mut res = conn.execute(
            "CREATE TABLE tiles (zoom_level integer, tile_column integer, tile_row integer, tile_data blob)",
            [],
        );
        assert!(res.is_ok());
        res = conn.execute(
            "CREATE TABLE metadata (name text, value text)",
            [],
        );
        assert!(res.is_ok());

        let z = 0;
        let x  = 0;
        let y  = 0;
        let tile_data:  &[u8] = &[0u8];

        conn.execute(
            "INSERT INTO tiles VALUES (?1, ?2, ?3, ?4)",
            (z, x, y, tile_data)
        ).unwrap();

        let repo = MBTilesTileRepository::new(conn);
        let tile = repo.get_tile(z, x, y).unwrap();

        assert_eq!(tile.zoom_level, z);
        assert_eq!(tile.tile_column, x);
        assert_eq!(tile.tile_row, y);
        assert_eq!(tile.tile_data, tile_data);
    }
}
