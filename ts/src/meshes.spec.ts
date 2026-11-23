import { readFileSync } from "node:fs";
import * as path from "node:path";
import { describe, expect, it } from "vitest";
import {
    columnValue,
    decodeTile,
    decodeTile3D,
    decodeTileColumns,
    decodeVectorTile,
    featureGeometry,
    GEOMETRY_TYPE,
    geometryStarts,
    isMeshes,
    isTrianglesOnly,
    MltGeometryType,
    mValueSlice,
    meshTriangles,
    tileGeoJson,
    toElevation,
} from "./index";
import { featureTablesToFeatureCollection } from "./vector/featureTablesToGeoJson";
import { compareWithTolerance } from "../../test/synthetic/synthetic-test-utils";

const REPO_ROOT = path.resolve(__dirname, "../..");
const SYNTHETIC_02 = path.join(REPO_ROOT, "test/synthetic/0x02");
const SYNTHETIC_01 = path.join(REPO_ROOT, "test/synthetic/0x01");
const CITYGML_FIXTURES = [
    path.join(REPO_ROOT, "test/fixtures/meshes/tiles/15/17294/10590.mlt"),
    path.join(REPO_ROOT, "test/fixtures/meshes/tiles/16/34588/21180.mlt"),
];

function readSynthetic02(stem: string): { mlt: Uint8Array; expected: Record<string, unknown> } {
    const mlt = new Uint8Array(readFileSync(path.join(SYNTHETIC_02, `${stem}.mlt`)));
    const expected = JSON.parse(readFileSync(path.join(SYNTHETIC_02, `${stem}.json`), "utf-8"));
    return { mlt, expected };
}

describe("MLT v2 3D Meshes, Z-axis, M-values, and Columnar APIs", () => {
    expect.addEqualityTesters([compareWithTolerance]);

    describe("CityGML v2 Meshes fixtures", () => {
        for (const fixturePath of CITYGML_FIXTURES) {
            const label = path.relative(REPO_ROOT, fixturePath).replaceAll(path.sep, "/");

            it(`decodes ${label} across decodeTileColumns, decodeTile3D, decodeTile, and tileGeoJson`, () => {
                const bytes = new Uint8Array(readFileSync(fixturePath));

                const colTile = decodeTileColumns(bytes);
                expect(colTile.layers.length).toBeGreaterThan(0);

                const tile3d = decodeTile3D(bytes);
                const tables = decodeTile(bytes, undefined, false);
                expect(tables.length).toBe(colTile.layers.length);

                const geojsonFromCols = tileGeoJson(bytes);
                const geojsonFromTables = featureTablesToFeatureCollection(tables);
                expect(geojsonFromTables).toEqual(geojsonFromCols);

                for (let l = 0; l < colTile.layers.length; l++) {
                    const layer = colTile.layers[l];
                    const table = tables[l];
                    expect(layer.version).toBe(2);
                    expect(table.version).toBe(2);

                    const { geometry } = layer;
                    expect(geometry.dimension).toBe(3);
                    expect(geometry.zStep).toBeDefined();
                    expect(table.zStep).toBe(geometry.zStep);
                    expect(table.heightMode).toBe(geometry.heightMode);

                    if (isMeshes(geometry)) {
                        expect(geometry.layout).toBe("meshes");
                        expect(geometry.meshes).toBe(true);
                        expect(isTrianglesOnly(geometry)).toBe(false);

                        const starts = geometryStarts(geometry);
                        expect(starts.featureGeometries.length).toBe(layer.featureCount + 1);

                        const layer3d = tile3d.layers[layer.name];
                        expect(layer3d).toBeDefined();
                        expect(layer3d.length).toBe(layer.featureCount);
                        expect(layer3d.zStep).toBe(geometry.zStep);

                        const tableFeatures = table.getFeatures();
                        expect(tableFeatures.length).toBe(layer.featureCount);

                        for (let f = 0; f < layer.featureCount; f++) {
                            expect(geometry.types[f]).toBe(MltGeometryType.Mesh);
                            expect(tableFeatures[f].geometry.type).toBe(GEOMETRY_TYPE.MESH);

                            const fg = featureGeometry(layer, f);
                            expect(fg.kind).toBe("mesh");
                            if (fg.kind !== "mesh") continue;

                            const vertCount = fg.vertices.length / 3;
                            expect(vertCount).toBeGreaterThanOrEqual(3);
                            expect(fg.triangles.length).toBeGreaterThanOrEqual(3);
                            expect(fg.triangles.length % 3).toBe(0);

                            for (let k = 0; k < fg.triangles.length; k++) {
                                expect(fg.triangles[k]).toBeGreaterThanOrEqual(0);
                                expect(fg.triangles[k]).toBeLessThan(vertCount);
                            }

                            const f3d = layer3d.feature(f);
                            const rings3d = f3d.loadGeometry();
                            expect(rings3d.length).toBe(1);
                            expect(rings3d[0].length).toBe(vertCount);
                            expect(f3d.loadTriangles()).toEqual(fg.triangles);
                            expect(() => f3d.loadPolygons()).toThrow(/mesh/i);

                            const elev = toElevation(rings3d[0][0][2], layer3d.zStep);
                            expect(Number.isFinite(elev)).toBe(true);
                        }
                    }
                }
            });
        }
    });

    describe("Synthetic v2 Mesh and scoped M-value tiles", () => {
        const meshStems = [
            "mesh_triangle",
            "mesh_roof_wall",
            "mesh_relative_to_ground",
            "mesh_mvalues_vertex",
            "mesh_mvalues_triangle",
            "mesh_mvalues_both",
        ];

        for (const stem of meshStems) {
            it(`matches expected GeoJSON and columnar invariants for ${stem}`, () => {
                const { mlt, expected } = readSynthetic02(stem);

                expect(tileGeoJson(mlt)).toEqual(expected);

                const colTile = decodeTileColumns(mlt);
                expect(colTile.layers.length).toBe(1);
                const layer = colTile.layers[0];
                expect(isMeshes(layer.geometry)).toBe(true);
                expect(layer.geometry.layout).toBe("meshes");

                const tile3d = decodeTile3D(mlt);
                const layer3d = tile3d.layers[layer.name];
                expect(layer3d.length).toBe(layer.featureCount);

                for (let f = 0; f < layer.featureCount; f++) {
                    const tris = meshTriangles(layer.geometry, f);
                    const f3d = layer3d.feature(f);
                    expect(f3d.loadTriangles()).toEqual(tris);

                    for (const vCol of layer.vertexValues) {
                        const s1 = mValueSlice(vCol, f);
                        const s2 = mValueSlice(layer, vCol, f);
                        expect(s2).toEqual(s1);
                        expect(f3d.loadVertexValues(vCol.name)).toEqual(s1 ?? undefined);
                        if (s1 !== null && "length" in s1) {
                            expect(s1.length).toBe(f3d.loadGeometry()[0].length);
                        }
                    }

                    for (const tCol of layer.triangleValues) {
                        const s1 = mValueSlice(tCol, f);
                        const s2 = mValueSlice(layer, tCol, f);
                        expect(s2).toEqual(s1);
                        expect(f3d.loadTriangleValues(tCol.name)).toEqual(s1 ?? undefined);
                        if (s1 !== null && "length" in s1) {
                            expect(s1.length).toBe(tris.length / 3);
                        }
                    }
                }
            });
        }

        it("preserves relativeToGround heightMode on 3D Z and Mesh layers", () => {
            const { mlt } = readSynthetic02("mesh_relative_to_ground");
            const layer = decodeTileColumns(mlt).layers[0];
            expect(layer.geometry.heightMode).toBe("relativeToGround");

            const tile3d = decodeTile3D(mlt);
            expect(tile3d.layers[layer.name].heightMode).toBe("relativeToGround");
            expect(tile3d.layers[layer.name].feature(0).heightMode).toBe("relativeToGround");

            const tables = decodeTile(mlt);
            expect(tables[0].heightMode).toBe("relativeToGround");
        });
    });

    describe("2D / 3D VectorTile wrapper, TessPolygons guards, and layer filtering", () => {
        it("decodes 2D and 3D non-mesh layers via decodeVectorTile and guards decodeTile3D", () => {
            const { mlt: zPolyMlt, expected: zPolyExpected } = readSynthetic02("z_poly_hole");
            expect(tileGeoJson(zPolyMlt)).toEqual(zPolyExpected);

            const vt = decodeVectorTile(zPolyMlt);
            const layerName = Object.keys(vt.layers)[0];
            const vtFeature = vt.layers[layerName].feature(0);
            expect(vtFeature.loadPolygons().length).toBe(1);
            expect(vtFeature.loadZ().length).toBeGreaterThan(0);

            const tile3d = decodeTile3D(zPolyMlt);
            const f3d = tile3d.layers[layerName].feature(0);
            expect(f3d.loadPolygons()[0][0][0].length).toBe(3);

            // 2D layer must throw in decodeTile3D
            const { mlt: pt2dMlt } = readSynthetic02("prop_i32");
            expect(() => decodeTile3D(pt2dMlt)).toThrow(/no z coordinates/i);

            // TessPolygons layer must throw in decodeTile3D and featureGeometry
            const { mlt: zTriMlt } = readSynthetic02("z_poly_hole_tri");
            expect(() => decodeTile3D(zTriMlt)).toThrow(/TessPolygons/i);
            const triLayer = decodeTileColumns(zTriMlt).layers[0];
            expect(isTrianglesOnly(triLayer.geometry)).toBe(true);
            expect(() => featureGeometry(triLayer, 0)).toThrow(/TessPolygons/i);
            expect(() => geometryStarts(triLayer.geometry)).toThrow(/TessPolygons/i);
        });

        it("decodes v1 (0x01) tiles through decodeTileColumns and tileGeoJson", () => {
            const v1Bytes = new Uint8Array(readFileSync(path.join(SYNTHETIC_01, "props_shared_dict.mlt")));
            const v1Expected = JSON.parse(readFileSync(path.join(SYNTHETIC_01, "props_shared_dict.json"), "utf-8"));
            const colTile = decodeTileColumns(v1Bytes);
            expect(colTile.layers[0].version).toBe(1);
            expect(colTile.layers[0].properties.length).toBeGreaterThan(0);
            expect(columnValue(colTile.layers[0].properties[0], 0)).not.toBeNull();
            expect(tileGeoJson(v1Bytes)).toEqual(v1Expected);
        });

        it("filters layers via options.layers without decoding skipped layers", () => {
            const bytes = new Uint8Array(readFileSync(CITYGML_FIXTURES[0]));
            const allLayers = decodeTileColumns(bytes).layers;
            const firstName = allLayers[0].name;

            const filtered = decodeTileColumns(bytes, { layers: [firstName] });
            expect(filtered.layers.map((l) => l.name)).toEqual([firstName]);

            const empty = decodeTileColumns(bytes, { layers: ["__non_existent_layer__"] });
            expect(empty.layers).toEqual([]);
        });

        it("enforces maxBytes decompression budget including dictionary string expansion", () => {
            const bytes = new Uint8Array(readFileSync(CITYGML_FIXTURES[0]));
            expect(() => decodeTileColumns(bytes, { maxBytes: 128 })).toThrow(RangeError);

            const { mlt: dictMlt } = readSynthetic02("props_str_dict");
            expect(() => decodeTileColumns(dictMlt, { maxBytes: 8 })).toThrow(RangeError);
        });
    });
});
