import {decodeTileAndMetadata, FeatureTable, filter} from "../../../src";
import fs from "fs";
import {ExpressionSpecification} from "@maplibre/maplibre-gl-style-spec";
import Path from "path";
import {VectorTile} from "@mapbox/vector-tile";
import Pbf from "pbf";


const omtOptimizedPretessellatedMltDir = "./test/data/omt/optimized/mlt/pre-tessellated";
const omtOptimizedMvtDir = "./test/data/omt/optimized/mvt";
const mltMetadata = fs.readFileSync(Path.join(omtOptimizedPretessellatedMltDir, "tileset.pbf"));
const tileIds = new Map<number, string>([
    [0, "0_0_0"],
    [1, "1_0_0"],
    [2, "2_2_1"],
    [3, "3_4_2"],
    [6, "6_32_21"],
]);
const decodedMltTiles = new Map<number, FeatureTable[]>();
const decodedMvtTiles = new Map<number, VectorTile>();

for(const [zoom, tileId] of tileIds){
    const encodedMlt = fs.readFileSync(Path.join(omtOptimizedPretessellatedMltDir, `${tileId}.mlt`));
    const decodedMlt = decodeTileAndMetadata(encodedMlt, mltMetadata);
    decodedMltTiles.set(zoom, decodedMlt);
    const encodedMvt7 = fs.readFileSync(Path.join(omtOptimizedMvtDir, `${tileId}.mvt`));
    const buf = new Pbf(encodedMvt7);
    const decodedMvt = new VectorTile(buf);
    decodedMvtTiles.set(zoom, decodedMvt);
}


describe("filter", () => {
    describe("based on a comparison expression", () => {
        it("with  equal instruction should return valid SelectionVector", () => {
            const zoom = 0;
            const sourceLayerName = "landcover";
            const layerFilter: ExpressionSpecification = [
                "==",
                "subclass",
                "glacier"
            ];
            const featureTable = decodedMltTiles.get(zoom).filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[mvtFilter[1]] === mvtFilter[2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues()).toEqual(expectedSelectionVector);
        });

        it("with  geometry equal instruction should return valid SelectionVector", () => {
            const layerFilter: ExpressionSpecification =   [
                "==",
                "$type",
                "Polygon"
            ];
            const sourceLayerName = "water";
            const featureTable = decodedMltTiles.get(3).filter(featureTable => featureTable.name === sourceLayerName)[0];
            const expectedSelectionVectorSize = 62;
            const expectedSelectionVector = [...Array(expectedSelectionVectorSize).keys()];

            const selectionVector = filter(featureTable, layerFilter);

            expect(selectionVector.limit).toEqual(expectedSelectionVectorSize);
            expect(selectionVector.selectionValues()).toEqual(expectedSelectionVector);
        });
    });

    describe("based on a compound expression", () => {
        it("with equal and unequal comparison instructions should return valid SelectionVector", () => {
            const zoom = 0;
            const sourceLayerName = "boundary";
            const layerFilter: ExpressionSpecification =  [
                "all",
                [
                    "!=",
                    "maritime",
                    1
                ],
                [
                    "==",
                    "disputed",
                    1
                ]
            ];
            const featureTable = decodedMltTiles.get(zoom).
                filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[layerFilter[1][1]] !== layerFilter[1][2]
                && feature.properties[layerFilter[2][1]] === layerFilter[2][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with unequal comparison instructions should return valid SelectionVector", () => {
            const zoom = 1;
            const sourceLayerName = "water";
            const layerFilter: ExpressionSpecification =  [
                "all",
                [
                    "!=",
                    "intermittent",
                    1
                ],
                [
                    "!=",
                    "brunnel",
                    "tunnel"
                ]
            ];
            const featureTable = decodedMltTiles.get(zoom).
                filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[layerFilter[1][1]] !== layerFilter[1][2]
                && feature.properties[layerFilter[2][1]] !== layerFilter[2][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with comparison instructions should return valid SelectionVector", () => {
            const zoom = 0;
            const sourceLayerName = "boundary";
            const layerFilter: ExpressionSpecification = [
                "all",
                [
                    ">=",
                    "admin:level",
                    3
                ],
                [
                    "<=",
                    "admin:level",
                    8
                ],
                [
                    "!=",
                    "maritime",
                    1
                ]
            ];
            const featureTable = decodedMltTiles.get(zoom).filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[layerFilter[1][1].replace(":", "_")] >= layerFilter[1][2]
                && feature.properties[layerFilter[2][1].replace(":", "_")] <= layerFilter[2][2]
                && feature.properties[layerFilter[3][1]] != layerFilter[3][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with equal and unequal comparison instructions should return valid SelectionVector", () => {
            const zoom = 1;
            const sourceLayerName = "boundary";
            const layerFilter: ExpressionSpecification =  ["all",
                [
                    "==",
                    "admin:level",
                    2
                ],
                [
                    "!=",
                    "maritime",
                    1
                ],
                [
                    "!=",
                    "disputed",
                    1
                ]
            ];
            const featureTable = decodedMltTiles.get(zoom).filter(featureTable =>
                featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[layerFilter[1][1].replace(":", "_")] === layerFilter[1][2]
                && feature.properties[layerFilter[2][1]] !== layerFilter[2][2]
                && feature.properties[layerFilter[3][1]] !== layerFilter[3][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with two unequal comparison instructions should return valid SelectionVector", () => {
            const zoom = 1;
            const sourceLayerName = "water";
            const layerFilter: ExpressionSpecification =  [
                "all",
                [
                    "!=",
                    "intermittent",
                        1
                ],
                [
                    "!=",
                    "brunnel",
                    "tunnel"
                ]
            ];
            const featureTable = decodedMltTiles.get(zoom).filter(featureTable =>
                featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[layerFilter[1][1]] !== layerFilter[1][2]
                && feature.properties[layerFilter[2][1]] !== layerFilter[2][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
    });

        it("with one equal and two unequal comparison instructions should return valid SelectionVector", () => {
            const zoom = 0;
            const sourceLayerName = "boundary";
            const layerFilter: ExpressionSpecification =  [
                "all",
                [
                    "==",
                    "admin:level",
                    2
                ],
                [
                    "!=",
                    "maritime",
                    1
                ],
                [
                    "!=",
                    "disputed",
                    1
                ]
            ];
            const featureTable = decodedMltTiles.get(zoom).
                filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature => feature.properties[layerFilter[1][1].replace(":", "_")] === layerFilter[1][2]
                && feature.properties[layerFilter[2][1]] !== layerFilter[2][2]
                && feature.properties[layerFilter[3][1]] !== layerFilter[3][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with geometry, false match, match and unequal comparison instructions should return valid SelectionVector", () => {
            const zoom = 6;
            const sourceLayerName = "transportation";
            const layerFilter: ExpressionSpecification = [
                "all",
                [
                    "==",
                    "$type",
                    "LineString"
                ],
                [
                    "!in",
                    "brunnel",
                    "bridge",
                    "tunnel"
                ],
                [
                    "in",
                    "class",
                    "primary"
                ],
                [
                    "!=",
                    "ramp",
                    1
                ]
            ] as any;
            const featureTable = decodedMltTiles.get(zoom).
                filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature =>
                feature.type === 2 &&
                !["bridge", "tunnel"].includes(feature.properties[layerFilter[2][1]]) &&
                feature.properties[layerFilter[3][1]] === layerFilter[3][2] &&
                feature.properties[layerFilter[4][1]] !== layerFilter[4][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with transportation layer equal and geometry type filter", () => {
            const zoom = 6;
            const sourceLayerName = "transportation";
            const layerFilter: ExpressionSpecification =
                ["all", ["==", "$type", "LineString"], ["==", "class", "motorway"]] as any;
            const featureTable = decodedMltTiles.get(zoom).
                filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature =>
                feature.type === 2 && feature.properties[layerFilter[2][1]] === layerFilter[2][2];
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector);
        });

        it("with transportation layer filter geometry and match filter", () => {
            const zoom = 6;
            const sourceLayerName = "transportation";
            const layerFilter: ExpressionSpecification =
                ["all", ["==", "$type", "LineString"], ["in", "class", "trunk", "primary"]] as any;
            const featureTable = decodedMltTiles.get(zoom).
                filter(featureTable => featureTable.name === sourceLayerName)[0];

            const selectionVector = filter(featureTable, layerFilter);

            const mvtFilter = feature =>
                feature.type === 2 && ["trunk", "primary"].includes(feature.properties[layerFilter[2][1]]);
            const expectedSelectionVector = filterMvtLayer(zoom, sourceLayerName, mvtFilter);
            expect(selectionVector.limit).toEqual(expectedSelectionVector.length);
            expect(selectionVector.selectionValues().slice(0, selectionVector.limit)).toEqual(expectedSelectionVector)
        });
    });
});

function filterMvtLayer(zoom: number, sourceLayerName: string, filter: (feature) => boolean) {
    const selectionVector = [];
    const mvt = decodedMvtTiles.get(zoom);
    const layer = mvt.layers[sourceLayerName]
    for (let i = 0; i < layer.length; i++) {
        const feature = layer.feature(i);
        if(filter(feature)){
            selectionVector.push(i);
        }
    }
    return selectionVector;
}
