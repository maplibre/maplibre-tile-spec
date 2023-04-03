import zlib from "zlib";
const {VectorTile} = require("@mapbox/vector-tile");
const Protobuf = require("pbf");
import MbTilesRepository from "../util/mbTilesRepository";
import {MVTLayer} from "../util/mvtLayer";
import {GeometryType, LineString, Point} from "../util/geometry";
import {zOrder} from "../encodings/spaceFillingCurveUtils";

/*
* Test size of a tile in zoom level 4
* -> Dominant layer -> transportation
* -> Id
*   -> Aufsteigend nach Id sortieren -> aber redundante Werte nicht nacheindander
*   -> Delta encoding und RLE Bitpacking hybrid
*
* -> RLE encoding
* -> Geometry type
* -> Topology
*   -> Colum oriented -> num LineString, numVertices
*   -> RLE bitpacking hybrid
* -> Index
*   -> Delta Coding
*   -> PFOR für bitpacking
* -> VertexBuffer
*   -> Order vertices on SFC
*   -> delta code
*   -> PFOR für bitpacking
* -> BitVector
* -> Properties
* */

interface LineStringTopology{
    numVertices: number;
}

interface PolygonTopology{
    numRings: number;
    numLineStrings: number[];
}

interface LayerGeometries{
    topology?: LineStringTopology | PolygonTopology[];
    geometryTypes: GeometryType[];
    vertices: Point[];
    indices?: number[];
}

const mbTilesFileName = "C:\\mapdata\\europe.mbtiles";
const tileIndex = {x: 8, y: 5, z: 4};
//const tileIndex = {x: 8718, y: 5685, z: 14};
//const tileIndex = {x: 16, y: 10, z: 5};
//const tileIndex = {x: 33, y: 21, z: 6};
//const tileIndex = {x: 536, y: 346, z: 10};
//const tileIndex = {x: 134, y: 86, z: 8};
//const tileIndex = {x: 272, y: 177, z: 9};
//const tileIndex = {x: 1, y: 0, z: 1};
//const tileIndex = {x: 2, y: 1, z: 2};
//const tileIndex = {x: 4, y: 2, z: 3};

(async () => {
    const mbTilesRepository = await MbTilesRepository.create(mbTilesFileName);
    const gzipCompressedMvtTile = await mbTilesRepository.getTile(tileIndex);
    const mvtTile = zlib.unzipSync(gzipCompressedMvtTile);

    const vectorTile = new VectorTile(new Protobuf(mvtTile));
    const tile = convertTile(vectorTile);
    console.info(tile);

    //TODO: encode Ids
    const geometries = convertGeometry(tile);
    console.info(geometries);

})();

function convertGeometry(tile: Map<string, MVTLayer>): Map<string, LayerGeometries>{
    const layersGeometries = new Map<string, LayerGeometries>();
    for(const [layerName, layer] of tile){
        /*
        * A layer can have different geometry types e.g. LineString and MultiLineString.
        * This can lead to very sparse columns.
        * Example with LineString and MultiLineString where NumLineString can be sparse
        * -> NumLineString, NumVertices
        * */
        const topology = [];
        const geometryTypes: GeometryType[] = [];
        const vertices: Point[] = [];
        const indices: number[] = [];

        for(const feature of layer.features){
            const geometryType = feature.type;
            if(geometryTypes.length > 0 && geometryTypes.at(-1) !== geometryType){
                //throw new Error("Different geometry types in a layer a currently not supported.");
            }
            geometryTypes.push(geometryType);

            switch (geometryType){
                case GeometryType.Point:
                    vertices.push(feature.geometry[0][0]);
                    break;
                case GeometryType.LineString:{
                    const lineString = feature.geometry[0];
                    const numVertices = lineString.length;
                    topology.push({numVertices});

                    /* Use Indexed Coordinate Encoding (ICE) for the transportation layer */
                    //if(layerName === "transportation"){
                    if(layerName === "boundary"){
                        /* Add to vertices to dictionary and sort on Z-order curve */
                        addToLineStringDictionary(lineString, vertices, indices);
                    }
                    else{
                        vertices.push(...lineString);
                    }
                    break;
                }
                case GeometryType.Polygon:{
                    const geometry = feature.geometry;
                    const numRings = geometry.length;
                    const numVertices = geometry.map(rings => rings.length);
                    topology.push({numRings, numVertices});
                    vertices.push(...geometry.flatMap(ring => ring));
                    break;
                }
                default:
                    throw new Error("The specified geometry type is not supported.");
            }
        }

        layersGeometries.set(layerName, {topology, geometryTypes, vertices, indices});
    }

    return layersGeometries;
}

/*
* Sort vertices on a Z-Order curve
* */
function addToLineStringDictionary(lineString: LineString, vertices: Point[], indices: number[]){
    for(const vertex of lineString){
        const storedVertexIndex = vertices.findIndex(v => v.x === vertex.x && v.y === vertex.y);
        if(storedVertexIndex === -1){
            const zOrderIndex = zOrder(vertex);
            const vertexIndex = vertices.findIndex(v => zOrderIndex < zOrder(v));
            if(vertexIndex === -1){
                indices.push(vertices.length);
                vertices.push(vertex);
            }
            else{
                vertices.splice(vertexIndex, 0, vertex);
                indices.push(vertexIndex);
                for(let i = 0; i < indices.length; i++){
                    const currentVertexIndex = indices[i];
                    if(currentVertexIndex > vertexIndex){
                        indices[i] = currentVertexIndex + 1;
                    }
                }
            }
        }
        else{
            indices.push(storedVertexIndex);
        }
    }
}

/*
* Sort the features of the layers based on the id for efficient delta encoding.
* */
function convertTile(tile): Map<string, MVTLayer>{
    const layers = new Map<string, MVTLayer>();
    for(const [name, layer] of Object.entries(tile.layers) as any) {
        const features = [];
        let type;
        for (let i = 0; i < layer.length; i++) {
            const feature = layer.feature(i);
            let id = feature.id;

            //TODO: handle this case -> why are the ids out of a long range?
            /*if(id > 1_446_744_073_700_112_000){
                id = id - 18_446_744_000_000_000_000
                //console.info("id is too large");
            }*/

            type = feature.type;
            const geometry = feature.loadGeometry();
            const properties = feature.properties;
            features.push({id, type, geometry, properties});
        }

        layers.set(name, {type, features});
    }

    return layers;
}


