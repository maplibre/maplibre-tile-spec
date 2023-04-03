import {GeometryType, Point} from "../util/geometry";

const { VectorTile } = require("@mapbox/vector-tile");
const Protobuf = require("pbf");
const fs = require("fs");
import zlib from "zlib";
import { MVTFeature, MVTLayer } from "../util/mvtLayer";
import MbTilesRepository from "../util/mbTilesRepository";

/**
 * Point -> Poi Zoom 14
 * Point -> Place Zoom 14
 * LineString -> Transportation Zoom 5
 * LineString -> Boundary Zoom 4
 * Polygon -> Building Zoom 14
 * Topology
 *  -> Zoom 4 Boundary
 *  -> Zoom 5 Transportation
 * Id
 *  -> Zoom 4 Boundary
 *  -> Zoom 5 Transportation
 * Variations
 * -> Geometry
 *  -> sorted Space Filling Curve, ...
 *  -> unsorted -> delta between vertices of a feature -> sorted by id
 * -> id
 *  -> sorted
 *  -> unsorted -> sorted by geometry
 * -> topology
 *  -> sorted
 *  -> unsorted -> sorted by id
 *
 *  File structure
 *  -> coordinates unsorted
 *  -> coordinates sorted
 *  -> delta coded per feature -> unsorted
 *  -> delta coded also between feature -> sorted
 *  -> id sorted
 *  -> id unsorted
 *  -> topology -> structure still no clear
 */

const mbTilesFileName = "C:\\mapdata\\europe.mbtiles";
/* Tiles in munich */
const tileIndices = [
    { x: 1, y: 0, z: 1 },
    { x: 8, y: 5, z: 4 },
    { x: 16, y: 10, z: 5 },
    { x: 33, y: 21, z: 6 },
    { x: 8718, y: 5685, z: 14 },
];

/**
 * The following dominant properties of a vector tile should be tested
 * Point -> Poi Zoom 14
 * Point -> Place Zoom 14
 * LineString -> Transportation Zoom 5
 * LineString -> Boundary Zoom 4
 * Polygon -> Building Zoom 14
 * Topology
 *  -> Zoom 4 Boundary
 *  -> Zoom 5 Transportation
 * Id
 *  -> Zoom 4 Boundary
 *  -> Zoom 5 Transportation
 */

enum PropertyType {
    ID = "Id",
    GEOMETRY = "Geometry",
    TOPOLOGY = "Topology",
}

interface LayerData {
    layerName: string;
    propertyType: PropertyType;
    geometryType?: GeometryType;
}

const testDataDescriptions = new Map<number, LayerData[]>([
    [1, [{ layerName: "boundary", propertyType: PropertyType.GEOMETRY, geometryType: GeometryType.LineString}]],
    [4, [{ layerName: "boundary", propertyType: PropertyType.ID }]],
    [5, [{ layerName: "transportation", propertyType: PropertyType.ID }]],
    [6, [{ layerName: "transportation", propertyType: PropertyType.ID }]],
    [
        14,
        [
            { layerName: "poi", propertyType: PropertyType.ID },
            { layerName: "poi", propertyType: PropertyType.GEOMETRY, geometryType: GeometryType.Point },
        ],
    ],
]);

/*
 * Generates test data to compare the different integer compression algorithms regarding
 * decoding speed and compression ratio for the dominant integers of a vector tile.
 */
(async () => {
    const mbTilesRepository = await MbTilesRepository.create(mbTilesFileName);

    for (const tileIndex of tileIndices) {
        if (testDataDescriptions.has(tileIndex.z)) {
            const gzipCompressedMvtTile = await mbTilesRepository.getTile(tileIndex);
            const mvtTile = zlib.unzipSync(gzipCompressedMvtTile);

            const vectorTile = new VectorTile(new Protobuf(mvtTile));
            const mvtLayers = convertTile(vectorTile);

            const testDataDescription = testDataDescriptions.get(tileIndex.z);
            for (const [name, layer] of mvtLayers) {
                for (const propertyDescription of testDataDescription) {
                    if (propertyDescription.layerName === name) {
                        switch (propertyDescription.propertyType) {
                            case PropertyType.ID: {
                                const [ids, sortedIds, sortedDeltaIds, deltaCodedUnsortedIds] = getIds(layer);
                                const idUnsortedFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_id_unsorted.json`;
                                const idSortedFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_id_sorted.json`;
                                const idSortedDeltaFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_id_sorted_delta.json`;
                                const idUnsortedDeltaFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_id_unsorted_delta.json`;

                                saveTestData(idSortedFileName, sortedIds);
                                saveTestData(idSortedDeltaFileName, sortedDeltaIds);
                                saveTestData(idUnsortedFileName, ids);
                                saveTestData(idUnsortedDeltaFileName, deltaCodedUnsortedIds);
                                break;
                            }
                            case PropertyType.GEOMETRY:
                                if (propertyDescription.geometryType === GeometryType.Point) {
                                    const [vertices, sortedVertices, sortedDeltaCodedVertices] = getPoints(layer);
                                    const pointUnsortedFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_point_unsorted.json`;
                                    const pointSortedFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_point_sorted.json`;
                                    const pointSortedDeltaFileName = `${propertyDescription.layerName}_zoom${tileIndex.z}_point_sorted_delta.json`;

                                    saveTestData(pointUnsortedFileName, vertices);
                                    saveTestData(pointSortedFileName, sortedVertices);
                                    saveTestData(pointSortedDeltaFileName, sortedDeltaCodedVertices);
                                }
                                break;
                            case PropertyType.TOPOLOGY:
                                break;
                        }
                    }
                }
            }
        }
    }

    await mbTilesRepository.dispose();
})();

function getPoints(layer: MVTLayer): [vertices: Point[], sortedVertices: Point[], deltaCodedVertices: Point[]] {
    const vertices = layer.features.map((f) => f.geometry[0][0]);
    const sortedVertices = [...vertices]
        .map((vertex) => {
            return { vertex, index: zOrder(vertex) };
        })
        .sort((v1, v2) => v1.index - v2.index)
        .map((vertex) => vertex.vertex);
    const deltaCodedVertices = sortedVertices.reduce((p, c, i) => {
        const pVertex = i === 0 ? { x: 0, y: 0 } : sortedVertices[i - 1];
        const deltaX = c.x - pVertex.x;
        const deltaY = c.y - pVertex.y;
        p.push({ x: deltaX, y: deltaY });
        return p;
    }, []);

    return [vertices, sortedVertices, deltaCodedVertices];
}

/*
 * https://gist.github.com/mourner/a7c6ea76f450bb92e316
 * */
//TODO: verify results of this calculation
function zOrder(point) {
    let x = Math.round((1024 * (fromZigZag(point.x) + 1024)) / 6144);
    let y = Math.round((1024 * (fromZigZag(point.y) + 1024)) / 6144);
    x = (x | (x << 8)) & 0x00ff00ff;
    x = (x | (x << 4)) & 0x0f0f0f0f;
    x = (x | (x << 2)) & 0x33333333;
    x = (x | (x << 1)) & 0x55555555;
    y = (y | (y << 8)) & 0x00ff00ff;
    y = (y | (y << 4)) & 0x0f0f0f0f;
    y = (y | (y << 2)) & 0x33333333;
    y = (y | (y << 1)) & 0x55555555;
    return x | (y << 1);
}

function fromZigZag(n) {
    return n % 2 === 0 ? n / 2 : -(n + 1) / 2;
}

function convertTile(tile): Map<string, MVTLayer> {
    const layers = new Map<string, MVTLayer>();
    for (const [name, layer] of Object.entries(tile.layers) as any) {
        const features: MVTFeature[] = [];
        for (let i = 0; i < layer.length; i++) {
            const feature = layer.feature(i);
            const id = feature.id;
            const type = feature.type;
            const geometry = feature.loadGeometry();
            const properties = feature.properties;
            features.push({ id, type, geometry, properties });
        }

        const type = features[0].type;
        layers.set(name, { type, features });
    }

    return layers;
}

function getIds(layer: MVTLayer): [ids: number[], sortedIds: number[], deltaCodedIds: number[], deltaCodedUnsortedIds: number[]] {
    const ids = layer.features.map((f) => f.id);
    const sortedIds = [...ids].sort((id1, id2) => id1 - id2);
    const deltaCodedIds = sortedIds.reduce((p, c, i) => {
        p.push(c - (i === 0 ? 0 : sortedIds[i - 1]));
        return p;
    }, []);
    const deltaCodedUnsortedIds = ids.reduce((p, c, i) => {
        p.push(c - (i === 0 ? 0 : ids[i - 1]));
        return p;
    }, []);
    return [ids, sortedIds, deltaCodedIds, deltaCodedUnsortedIds];
}

function saveTestData(fileName: string, testData: unknown) {
    const json = JSON.stringify(testData);
    fs.writeFileSync(`./data/${fileName}`, json);
}
