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
    return {
        mlt: new Uint8Array(readFileSync(path.join(SYNTHETIC_02, `${stem}.mlt`))),
        expected: JSON.parse(readFileSync(path.join(SYNTHETIC_02, `${stem}.json`), "utf-8")),
    };
}

describe("MLT v2 3D Meshes, Z-axis, M-values, and Columnar APIs", () => {
    expect.addEqualityTesters([compareWithTolerance]);

    for (const fixturePath of CITYGML_FIXTURES) {
        const label = path.relative(REPO_ROOT, fixturePath).replaceAll(path.sep, "/");
        it(`decodes ${label} across decodeTileColumns, decodeTile3D, decodeTile, and tileGeoJson`, () => {
            const bytes = new Uint8Array(readFileSync(fixturePath));
            const colTile = decodeTileColumns(bytes);
            const tile3d = decodeTile3D(bytes);
            const tables = decodeTile(bytes, undefined, false);
            expect(colTile.layers.length).toBeGreaterThan(0);
            expect(tables.length).toBe(colTile.layers.length);
            expect(featureTablesToFeatureCollection(tables)).toEqual(tileGeoJson(bytes));

            for (let l = 0; l < colTile.layers.length; l++) {
                const layer = colTile.layers[l];
                const table = tables[l];
                const { geometry } = layer;
                expect(layer.version).toBe(2);
                expect(table.version).toBe(2);
                expect(geometry.dimension).toBe(3);
                expect(table.zStep).toBe(geometry.zStep);
                expect(table.heightMode).toBe(geometry.heightMode);
                if (!isMeshes(geometry)) continue;

                expect(geometry.layout).toBe("meshes");
                expect(isTrianglesOnly(geometry)).toBe(false);
                expect(geometryStarts(geometry).featureGeometries.length).toBe(layer.featureCount + 1);

                const layer3d = tile3d.layers[layer.name];
                const tableFeatures = table.getFeatures();
                expect(layer3d.length).toBe(layer.featureCount);
                expect(tableFeatures.length).toBe(layer.featureCount);

                for (let f = 0; f < layer.featureCount; f++) {
                    expect(geometry.types[f]).toBe(MltGeometryType.Mesh);
                    expect(tableFeatures[f].geometry.type).toBe(GEOMETRY_TYPE.MESH);
                    const fg = featureGeometry(layer, f);
                    if (fg.kind !== "mesh") throw new Error("expected mesh");
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
                    expect(rings3d[0].length).toBe(vertCount);
                    expect(f3d.loadTriangles()).toEqual(fg.triangles);
                    expect(() => f3d.loadPolygons()).toThrow(/mesh/i);
                    expect(Number.isFinite(toElevation(rings3d[0][0][2], layer3d.zStep))).toBe(true);
                }
            }
        });
    }

    for (const stem of [
        "mesh_triangle",
        "mesh_roof_wall",
        "mesh_relative_to_ground",
        "mesh_mvalues_vertex",
        "mesh_mvalues_triangle",
        "mesh_mvalues_both",
    ]) {
        it(`matches expected GeoJSON and columnar invariants for ${stem}`, () => {
            const { mlt, expected } = readSynthetic02(stem);
            expect(tileGeoJson(mlt)).toEqual(expected);
            const layer = decodeTileColumns(mlt).layers[0];
            const layer3d = decodeTile3D(mlt).layers[layer.name];
            expect(isMeshes(layer.geometry)).toBe(true);

            for (let f = 0; f < layer.featureCount; f++) {
                const tris = meshTriangles(layer.geometry, f);
                const f3d = layer3d.feature(f);
                expect(f3d.loadTriangles()).toEqual(tris);
                for (const vCol of layer.vertexValues) {
                    const s1 = mValueSlice(vCol, f);
                    expect(mValueSlice(layer, vCol, f)).toEqual(s1);
                    expect(f3d.loadVertexValues(vCol.name)).toEqual(s1 ?? undefined);
                }
                for (const tCol of layer.triangleValues) {
                    const s1 = mValueSlice(tCol, f);
                    expect(mValueSlice(layer, tCol, f)).toEqual(s1);
                    expect(f3d.loadTriangleValues(tCol.name)).toEqual(s1 ?? undefined);
                }
            }
        });
    }

    it("preserves relativeToGround, VectorTile/3D guards, v1 tiles, filtering, and maxBytes", () => {
        const { mlt: relMlt } = readSynthetic02("mesh_relative_to_ground");
        const relLayer = decodeTileColumns(relMlt).layers[0];
        expect(relLayer.geometry.heightMode).toBe("relativeToGround");
        expect(decodeTile3D(relMlt).layers[relLayer.name].feature(0).heightMode).toBe("relativeToGround");
        expect(decodeTile(relMlt)[0].heightMode).toBe("relativeToGround");

        const { mlt: zPolyMlt, expected: zPolyExpected } = readSynthetic02("z_poly_hole");
        expect(tileGeoJson(zPolyMlt)).toEqual(zPolyExpected);
        const vt = decodeVectorTile(zPolyMlt);
        const ln = Object.keys(vt.layers)[0];
        expect(vt.layers[ln].feature(0).loadPolygons().length).toBe(1);
        expect(vt.layers[ln].feature(0).loadZ().length).toBeGreaterThan(0);
        expect(decodeTile3D(zPolyMlt).layers[ln].feature(0).loadPolygons()[0][0][0].length).toBe(3);

        expect(() => decodeTile3D(readSynthetic02("prop_i32").mlt)).toThrow(/no z coordinates/i);

        const { mlt: zTriMlt } = readSynthetic02("z_poly_hole_tri");
        const triLayer = decodeTileColumns(zTriMlt).layers[0];
        expect(isTrianglesOnly(triLayer.geometry)).toBe(true);
        expect(() => decodeTile3D(zTriMlt)).toThrow(/TessPolygons/i);
        expect(() => featureGeometry(triLayer, 0)).toThrow(/TessPolygons/i);
        expect(() => geometryStarts(triLayer.geometry)).toThrow(/TessPolygons/i);

        const v1Bytes = new Uint8Array(readFileSync(path.join(SYNTHETIC_01, "props_shared_dict.mlt")));
        const v1Expected = JSON.parse(readFileSync(path.join(SYNTHETIC_01, "props_shared_dict.json"), "utf-8"));
        const v1Layer = decodeTileColumns(v1Bytes).layers[0];
        expect(v1Layer.version).toBe(1);
        expect(columnValue(v1Layer.properties[0], 0)).not.toBeNull();
        expect(tileGeoJson(v1Bytes)).toEqual(v1Expected);

        const cityBytes = new Uint8Array(readFileSync(CITYGML_FIXTURES[0]));
        const firstName = decodeTileColumns(cityBytes).layers[0].name;
        expect(decodeTileColumns(cityBytes, { layers: [firstName] }).layers.map((l) => l.name)).toEqual([firstName]);
        expect(decodeTileColumns(cityBytes, { layers: ["__missing__"] }).layers).toEqual([]);

        expect(() => decodeTileColumns(cityBytes, { maxBytes: 128 })).toThrow(RangeError);
        expect(() => decodeTileColumns(readSynthetic02("props_str_dict").mlt, { maxBytes: 8 })).toThrow(RangeError);
    });
});
