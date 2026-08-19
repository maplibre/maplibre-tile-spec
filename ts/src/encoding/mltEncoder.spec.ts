import { describe, it, expect } from "vitest";
import decodeTile from "../mltDecoder";
import { encodeTile, type EncodeOptions, type Feature, type FeatureGeometry } from "./mltEncoder";
import { GEOMETRY_TYPE } from "../vector/geometry/geometryType";

/** Encodes one layer, decodes it again, and returns the features in a comparable plain shape. */
function roundTrip(features: Feature[], options: EncodeOptions = {}, extent = 4096) {
    const tile = encodeTile([{ name: "layer", features, extent }], options);
    const tables = decodeTile(tile, undefined, false);

    expect(tables).toHaveLength(1);
    expect(tables[0].name).toBe("layer");
    expect(tables[0].extent).toBe(extent);

    return tables[0].getFeatures().map((feature) => ({
        id: feature.id,
        type: feature.geometry.type,
        coordinates: feature.geometry.coordinates.map((ring) => ring.map((point) => [point.x, point.y])),
        properties: feature.properties,
    }));
}

describe("encodeTile - geometry", () => {
    it("round-trips a point", () => {
        const [decoded] = roundTrip([{ geometry: { type: "Point", coordinates: [3, 9] } }]);
        expect(decoded.type).toBe(GEOMETRY_TYPE.POINT);
        expect(decoded.coordinates).toEqual([[[3, 9]]]);
    });

    it("round-trips several points", () => {
        const decoded = roundTrip([
            { geometry: { type: "Point", coordinates: [1, 2] } },
            { geometry: { type: "Point", coordinates: [-3, 4] } },
        ]);
        expect(decoded.map((feature) => feature.coordinates)).toEqual([[[[1, 2]]], [[[-3, 4]]]]);
    });

    it("round-trips a multi-point", () => {
        const geometry: FeatureGeometry = {
            type: "MultiPoint",
            coordinates: [
                [1, 2],
                [3, 4],
            ],
        };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.type).toBe(GEOMETRY_TYPE.MULTIPOINT);
        expect(decoded.coordinates).toEqual([[[1, 2]], [[3, 4]]]);
    });

    it("round-trips a line string", () => {
        const geometry: FeatureGeometry = {
            type: "LineString",
            coordinates: [
                [0, 0],
                [5, 5],
                [10, 0],
            ],
        };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.type).toBe(GEOMETRY_TYPE.LINESTRING);
        expect(decoded.coordinates).toEqual([
            [
                [0, 0],
                [5, 5],
                [10, 0],
            ],
        ]);
    });

    it("round-trips a multi-line string", () => {
        const geometry: FeatureGeometry = {
            type: "MultiLineString",
            coordinates: [
                [
                    [0, 0],
                    [1, 1],
                ],
                [
                    [2, 2],
                    [3, 3],
                    [4, 4],
                ],
            ],
        };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.type).toBe(GEOMETRY_TYPE.MULTILINESTRING);
        expect(decoded.coordinates).toEqual(geometry.coordinates);
    });

    it("round-trips a polygon, closing the ring", () => {
        const geometry: FeatureGeometry = {
            type: "Polygon",
            coordinates: [
                [
                    [0, 0],
                    [10, 0],
                    [10, 10],
                    [0, 0],
                ],
            ],
        };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.type).toBe(GEOMETRY_TYPE.POLYGON);
        expect(decoded.coordinates).toEqual([
            [
                [0, 0],
                [10, 0],
                [10, 10],
                [0, 0],
            ],
        ]);
    });

    it("round-trips a polygon with a hole", () => {
        const geometry: FeatureGeometry = {
            type: "Polygon",
            coordinates: [
                [
                    [0, 0],
                    [10, 0],
                    [10, 10],
                    [0, 10],
                ],
                [
                    [2, 2],
                    [4, 2],
                    [4, 4],
                ],
            ],
        };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.coordinates).toHaveLength(2);
        expect(decoded.coordinates[0]).toHaveLength(5);
        expect(decoded.coordinates[1]).toHaveLength(4);
    });

    it("round-trips a multi-polygon whose rings differ in size", () => {
        const geometry: FeatureGeometry = {
            type: "MultiPolygon",
            coordinates: [
                [
                    [
                        [0, 0],
                        [10, 0],
                        [10, 10],
                        [0, 10],
                    ],
                    [
                        [2, 2],
                        [4, 2],
                        [4, 4],
                    ],
                ],
                [
                    [
                        [20, 20],
                        [22, 20],
                        [20, 22],
                    ],
                ],
            ],
        };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.type).toBe(GEOMETRY_TYPE.MULTIPOLYGON);
        expect(decoded.coordinates.map((ring) => ring.length)).toEqual([5, 4, 4]);
    });

    it("leaves a degenerate ring alone rather than trying to open it", () => {
        const geometry: FeatureGeometry = { type: "Polygon", coordinates: [[[7, 8]]] };
        const [decoded] = roundTrip([{ geometry }]);
        expect(decoded.coordinates[0][0]).toEqual([7, 8]);
    });

    it("round-trips features of mixed geometry types in one layer", () => {
        const decoded = roundTrip([
            { geometry: { type: "Point", coordinates: [1, 2] } },
            {
                geometry: {
                    type: "LineString",
                    coordinates: [
                        [3, 4],
                        [5, 6],
                    ],
                },
            },
        ]);
        expect(decoded.map((feature) => feature.type)).toEqual([GEOMETRY_TYPE.POINT, GEOMETRY_TYPE.LINESTRING]);
        expect(decoded[0].coordinates).toEqual([[[1, 2]]]);
        expect(decoded[1].coordinates).toEqual([
            [
                [3, 4],
                [5, 6],
            ],
        ]);
    });
});

describe("encodeTile - ids and properties", () => {
    const point: FeatureGeometry = { type: "Point", coordinates: [0, 0] };

    it("round-trips feature ids", () => {
        const decoded = roundTrip([
            { id: 7, geometry: point },
            { id: 9, geometry: point },
        ]);
        expect(decoded.map((feature) => feature.id)).toEqual([7, 9]);
    });

    it("round-trips string properties", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { name: "alpha" } },
            { geometry: point, properties: { name: "beta" } },
        ]);
        expect(decoded.map((feature) => feature.properties.name)).toEqual(["alpha", "beta"]);
    });

    it("round-trips integer properties", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { count: 1 } },
            { geometry: point, properties: { count: -2147483648 } },
        ]);
        expect(decoded.map((feature) => feature.properties.count)).toEqual([1, -2147483648]);
    });

    it("round-trips double properties", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { ratio: 1.5 } },
            { geometry: point, properties: { ratio: -0.25 } },
        ]);
        expect(decoded.map((feature) => feature.properties.ratio)).toEqual([1.5, -0.25]);
    });

    it("round-trips boolean properties", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { open: true } },
            { geometry: point, properties: { open: false } },
        ]);
        expect(decoded.map((feature) => feature.properties.open)).toEqual([true, false]);
    });

    it("round-trips 64-bit integer properties", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { big: 2n ** 40n } },
            { geometry: point, properties: { big: -(2n ** 40n) } },
        ]);
        expect(decoded.map((feature) => feature.properties.big)).toEqual([2n ** 40n, -(2n ** 40n)]);
    });

    it("round-trips a float column when the type is given", () => {
        const decoded = roundTrip(
            [
                { geometry: point, properties: { ratio: 1.5 } },
                { geometry: point, properties: { ratio: -2.25 } },
            ],
            { propertyTypes: { ratio: "float" } },
        );
        expect(decoded.map((feature) => feature.properties.ratio)).toEqual([1.5, -2.25]);
    });

    it("omits a property that is absent for a feature", () => {
        const decoded = roundTrip([{ geometry: point, properties: { name: "alpha" } }, { geometry: point }]);
        expect(decoded[0].properties.name).toBe("alpha");
        expect(decoded[1].properties.name).toBeUndefined();
    });

    it("round-trips several property columns at once", () => {
        const decoded = roundTrip([{ geometry: point, properties: { name: "alpha", count: 3, open: true } }]);
        expect(decoded[0].properties).toMatchObject({ name: "alpha", count: 3, open: true });
    });

    it("round-trips a property column that is null for every feature", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { note: null } },
            { geometry: point, properties: { note: null } },
        ]);
        expect(decoded.every((feature) => feature.properties.note === undefined)).toBe(true);
    });

    it("rejects a property column that mixes value types", () => {
        expect(() =>
            encodeTile([
                {
                    name: "layer",
                    features: [
                        { geometry: point, properties: { mixed: "a" } },
                        { geometry: point, properties: { mixed: 1 } },
                    ],
                },
            ]),
        ).toThrow(/mixes value types/);
    });
});

describe("encodeTile - nested properties", () => {
    const point: FeatureGeometry = { type: "Point", coordinates: [0, 0] };

    it("round-trips a map", () => {
        const decoded = roundTrip([{ geometry: point, properties: { nested: { a: "b", c: 1 } } }]);
        expect(decoded[0].properties.nested).toEqual({ a: "b", c: 1 });
    });

    it("round-trips a list", () => {
        const decoded = roundTrip([{ geometry: point, properties: { nested: ["a", 1, true] } }]);
        expect(decoded[0].properties.nested).toEqual(["a", 1, true]);
    });

    it("round-trips maps and lists nested in each other", () => {
        const value = { a: [1, { b: ["c", { d: 2.5 }] }] };
        const decoded = roundTrip([{ geometry: point, properties: { nested: value } }]);
        expect(decoded[0].properties.nested).toEqual(value);
    });

    it("takes a scalar as a whole feature value, so root shapes can differ per feature", () => {
        const decoded = roundTrip([
            { geometry: point, properties: { nested: "a" } },
            { geometry: point, properties: { nested: { b: "c" } } },
            { geometry: point, properties: { nested: ["d"] } },
        ]);
        expect(decoded.map((feature) => feature.properties.nested)).toEqual(["a", { b: "c" }, ["d"]]);
    });

    it("omits a nested property that is absent for a feature", () => {
        const decoded = roundTrip([{ geometry: point, properties: { nested: { a: "b" } } }, { geometry: point }]);
        expect(decoded[0].properties.nested).toEqual({ a: "b" });
        expect(decoded[1].properties.nested).toBeUndefined();
    });

    it("keeps an empty map apart from an absent value", () => {
        const decoded = roundTrip([{ geometry: point, properties: { nested: {} } }, { geometry: point }]);
        expect(decoded[0].properties.nested).toEqual({});
        expect(decoded[1].properties.nested).toBeUndefined();
    });

    it("writes a signed and an unsigned dictionary when the values need both", () => {
        const decoded = roundTrip([{ geometry: point, properties: { nested: [-2, 2n ** 63n] } }]);
        expect(decoded[0].properties.nested).toEqual([-2, 2n ** 63n]);
    });

    it("writes single-precision floats when asked to", () => {
        const decoded = roundTrip([{ geometry: point, properties: { nested: [1.5, -2.25] } }], {
            mapOptions: { singlePrecisionFloats: true },
        });
        expect(decoded[0].properties.nested).toEqual([1.5, -2.25]);
    });

    it("keeps a column nested when the type is pinned, so its features may differ in type", () => {
        const decoded = roundTrip(
            [
                { geometry: point, properties: { mixed: "a" } },
                { geometry: point, properties: { mixed: 1 } },
            ],
            { propertyTypes: { mixed: "map" } },
        );
        expect(decoded.map((feature) => feature.properties.mixed)).toEqual(["a", 1]);
    });

    it("rejects a null nested inside a value, which the format cannot express", () => {
        expect(() =>
            encodeTile([{ name: "layer", features: [{ geometry: point, properties: { nested: { a: null } } }] }]),
        ).toThrow(/Nested null values cannot be encoded/);
    });
});

describe("encodeTile - tiles and layers", () => {
    it("encodes several layers into one tile", () => {
        const point: FeatureGeometry = { type: "Point", coordinates: [1, 1] };
        const tile = encodeTile([
            { name: "first", features: [{ geometry: point }] },
            { name: "second", features: [{ geometry: point }] },
        ]);
        const tables = decodeTile(tile, undefined, false);

        expect(tables.map((table) => table.name)).toEqual(["first", "second"]);
    });

    it("uses the default extent when none is given", () => {
        const tile = encodeTile([{ name: "layer", features: [{ geometry: { type: "Point", coordinates: [0, 0] } }] }]);
        expect(decodeTile(tile, undefined, false)[0].extent).toBe(4096);
    });
});
