import { GEOMETRY_TYPE } from "../vector/geometry/geometryType";
import { ScalarType } from "../metadata/tileset/tilesetMetadata";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LengthType } from "../metadata/tile/lengthType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { PhysicalLevelTechnique } from "../metadata/tile/physicalLevelTechnique";
import { createStream } from "../decoding/decodingTestUtils";
import IntWrapper from "../decoding/intWrapper";
import { concatenateBuffers } from "./encodingUtils";
import { encodeVarintInt32, encodeVarintInt32Value, encodeZigZagInt32 } from "./integerEncodingUtils";
import { encodeChildCount, encodeFieldName, encodeTypeCode, scalarTypeCode } from "./embeddedTilesetMetadataEncoder";
import { ColumnTypeCode } from "../metadata/tileset/typeMap";
import { encodePlainStrings } from "./stringEncoder";
import { encodeMapPropertyColumn, type MapEncodingOptions, type MapValue } from "./mapPropertyEncoder";
import {
    encodeBooleanColumn,
    encodeBooleanNullableColumn,
    encodeDoubleColumn,
    encodeDoubleNullableColumn,
    encodeFloatColumn,
    encodeFloatNullableColumn,
    encodeInt32NoneColumn,
    encodeInt32NullableColumn,
    encodeInt64NoneColumn,
    encodeInt64NullableColumn,
    encodeUint32Column,
    encodeUint64Column,
    encodeUint64NullableColumn,
} from "./propertyEncoder";

/** A coordinate pair in tile-local units. */
export type Position = [number, number];

/**
 * The geometry of one feature, in the same nesting GeoJSON uses:
 * a point is a position, a line string a list of positions, a polygon a list of rings, and the
 * multi variants add one more level. Polygon rings are given without a repeated closing position.
 */
export type FeatureGeometry =
    | { type: "Point"; coordinates: Position }
    | { type: "MultiPoint"; coordinates: Position[] }
    | { type: "LineString"; coordinates: Position[] }
    | { type: "MultiLineString"; coordinates: Position[][] }
    | { type: "Polygon"; coordinates: Position[][] }
    | { type: "MultiPolygon"; coordinates: Position[][][] };

/**
 * A property value. Maps and lists of arbitrary depth are allowed and go out as a nested (MAP)
 * column; `null` means the property is absent for that feature.
 */
export type PropertyValue = MapValue | null;

export interface Feature {
    id?: number | bigint;
    geometry: FeatureGeometry;
    properties?: Record<string, PropertyValue>;
}

/** One feature table, the equivalent of a layer in the Java encoder's `LayerSource`. */
export interface Layer {
    name: string;
    features: Feature[];
    extent?: number;
}

/**
 * The physical type a property column is written as. Inferred per column when not given, which is
 * all the synthetic cases need; pass it explicitly to pin a width the values alone do not imply.
 *
 * `map` is the nested column, which holds maps, lists and scalars of any shape. It is inferred for
 * any column holding a map or a list, and can be pinned to keep a column nested even when the
 * values it happens to hold are all scalars.
 */
export type PropertyType = "boolean" | "int32" | "int64" | "uint64" | "float" | "double" | "string" | "map";

export interface EncodeOptions {
    /** Physical type per property name, for columns whose values do not imply one. */
    propertyTypes?: Record<string, PropertyType>;
    /** Dictionary widths for nested columns, which the values alone do not always imply. */
    mapOptions?: MapEncodingOptions;
}

const INT32_MIN = -(2 ** 31);
const INT32_MAX = 2 ** 31 - 1;
const UINT32_MAX = 2 ** 32 - 1;
const INT64_MAX = 2n ** 63n - 1n;
const DEFAULT_EXTENT = 4096;
const TAG = 1;

/**
 * Encodes layers into an MLT tile, the inverse of `decodeTile`.
 *
 * This is the plainest encoding the format allows: values go out as varints and plain strings, with
 * no dictionaries, no RLE, no FastPFOR or FSST, no morton-coded or dictionary-encoded vertices and
 * no pre-tessellation. The result is larger than what the Java or Rust encoders produce but is
 * built only from the stream encoders already in this package, and decodes to the same features.
 *
 * Nested (MAP) columns are the one exception: the format stores them as dictionaries of scalars and
 * a stream of tokens, so there is no plainer form to write them in.
 */
export function encodeTile(layers: Layer[], options: EncodeOptions = {}): Uint8Array {
    return concatenateBuffers(...layers.map((layer) => encodeLayer(layer, options)));
}

/** Each layer is a self-contained, length-prefixed block, so tiles are concatenations of layers. */
function encodeLayer(layer: Layer, options: EncodeOptions): Uint8Array {
    const extent = layer.extent ?? DEFAULT_EXTENT;
    const propertyNames = collectPropertyNames(layer.features);
    const hasIds = layer.features.some((feature) => feature.id !== undefined);

    const columns: Uint8Array[] = [];
    const metadata: Uint8Array[] = [];

    if (hasIds) {
        const ids = encodeIdColumn(layer.features);
        metadata.push(encodeTypeCode(ids.typeCode));
        columns.push(ids.data);
    }

    metadata.push(encodeTypeCode(ColumnTypeCode.GEOMETRY));
    columns.push(encodeGeometryColumn(layer.features));

    for (const name of propertyNames) {
        const values = layer.features.map((feature) => feature.properties?.[name] ?? null);
        const type = options.propertyTypes?.[name] ?? inferPropertyType(name, values);

        if (type === "map") {
            const { data, numStreams } = encodeMapPropertyColumn([values], options.mapOptions);
            metadata.push(encodeTypeCode(ColumnTypeCode.MAP), encodeFieldName(name), encodeChildCount(0));
            columns.push(concatenateBuffers(encodeVarintValue(numStreams), data));
            continue;
        }

        const nullable = values.some((value) => value === null);
        metadata.push(encodeTypeCode(scalarTypeCode(scalarTypeOf(type), nullable)), encodeFieldName(name));
        columns.push(encodePropertyColumn(type, values, nullable));
    }

    const body = concatenateBuffers(
        encodeVarintValue(TAG),
        encodeFieldName(layer.name),
        encodeVarintValue(extent),
        encodeVarintValue(columns.length),
        ...metadata,
        ...columns,
    );

    return concatenateBuffers(encodeVarintValue(body.length), body);
}

/** Property columns appear in every feature's order of first appearance, so the layout is stable. */
function collectPropertyNames(features: Feature[]): string[] {
    const names: string[] = [];
    for (const feature of features) {
        for (const name of Object.keys(feature.properties ?? {})) {
            if (!names.includes(name)) names.push(name);
        }
    }
    return names;
}

function inferPropertyType(name: string, values: PropertyValue[]): PropertyType {
    const present = values.filter((value) => value !== null);
    if (present.length === 0) return "string";
    // One map or list anywhere in the column makes the whole column nested. The nested encoding also
    // takes plain scalars, so the features whose value is a scalar still fit.
    if (present.some((value) => typeof value === "object")) return "map";
    if (present.every((value) => typeof value === "boolean")) return "boolean";
    if (present.every((value) => typeof value === "string")) return "string";
    if (present.every((value) => typeof value === "bigint")) {
        // Values past the signed maximum only fit in the unsigned column.
        return present.some((value) => value > INT64_MAX) ? "uint64" : "int64";
    }
    if (present.every((value) => typeof value === "number")) {
        if (!present.every((value) => Number.isSafeInteger(value))) return "double";
        return present.every((value) => value >= INT32_MIN && value <= INT32_MAX) ? "int32" : "int64";
    }
    throw new Error(`Property "${name}" mixes value types, so no single column type fits it`);
}

function scalarTypeOf(type: Exclude<PropertyType, "map">): number {
    switch (type) {
        case "boolean":
            return ScalarType.BOOLEAN;
        case "int32":
            return ScalarType.INT_32;
        case "int64":
            return ScalarType.INT_64;
        case "uint64":
            return ScalarType.UINT_64;
        case "float":
            return ScalarType.FLOAT;
        case "double":
            return ScalarType.DOUBLE;
        case "string":
            return ScalarType.STRING;
    }
}

/**
 * Writes the id column and the type code describing it. Nullable ids always go out as 64-bit,
 * because only the 64-bit encoder has a nullable form.
 */
function encodeIdColumn(features: Feature[]): { typeCode: number; data: Uint8Array } {
    const ids = features.map((feature) => feature.id);
    const nullable = ids.some((id) => id === undefined);
    const wide = ids.some((id) => id !== undefined && BigInt(id) > BigInt(UINT32_MAX));

    if (nullable) {
        return {
            typeCode: ColumnTypeCode.ID | ColumnTypeCode.ID_LONG | ColumnTypeCode.ID_NULLABLE,
            data: encodeUint64NullableColumn(ids.map((id) => (id === undefined ? null : BigInt(id)))),
        };
    }
    if (wide) {
        return {
            typeCode: ColumnTypeCode.ID | ColumnTypeCode.ID_LONG,
            data: encodeUint64Column(BigUint64Array.from(ids, (id) => BigInt(id ?? 0))),
        };
    }
    return { typeCode: ColumnTypeCode.ID, data: encodeUint32Column(Uint32Array.from(ids, (id) => Number(id ?? 0))) };
}

/** Writes one non-nested column. Nested columns go through {@link encodeMapPropertyColumn}. */
function encodePropertyColumn(
    type: Exclude<PropertyType, "map">,
    values: PropertyValue[],
    nullable: boolean,
): Uint8Array {
    if (type === "string") {
        const strings = values.map((value) => (value === null ? null : String(value)));
        // The decoder is told how many streams the column holds; plain strings use a length and a
        // data stream, preceded by a presence stream when the column is nullable.
        return concatenateBuffers(encodeVarintValue(nullable ? 3 : 2), encodePlainStrings(strings));
    }

    if (type === "boolean") {
        const booleans = values.map((value) => (value === null ? null : Boolean(value)));
        return nullable ? encodeBooleanNullableColumn(booleans) : encodeBooleanColumn(booleans);
    }

    if (type === "int64" || type === "uint64") {
        const bigints = values.map((value) => (value === null ? null : BigInt(value as number | bigint)));
        if (type === "uint64") {
            return nullable ? encodeUint64NullableColumn(bigints) : encodeUint64Column(BigUint64Array.from(bigints));
        }
        return nullable ? encodeInt64NullableColumn(bigints) : encodeInt64NoneColumn(BigInt64Array.from(bigints));
    }

    const numbers = values.map((value) => (value === null ? null : Number(value)));
    if (type === "int32") {
        return nullable ? encodeInt32NullableColumn(numbers) : encodeInt32NoneColumn(Int32Array.from(numbers));
    }
    if (type === "float") {
        return nullable ? encodeFloatNullableColumn(numbers) : encodeFloatColumn(Float32Array.from(numbers));
    }
    return nullable ? encodeDoubleNullableColumn(numbers) : encodeDoubleColumn(Float64Array.from(numbers));
}

/**
 * Writes the geometry column: a stream of geometry types, the nested length streams, then the
 * vertex buffer.
 *
 * Which length streams are written depends on the geometry types in the layer, because the decoder
 * reads each combination differently. With a GEOMETRIES stream present it takes the part count of a
 * line string to be implicit, so writing the full set unconditionally would lose their vertex
 * counts. The four combinations below are the ones the decoder recognises:
 *
 * - multi + polygon: GEOMETRIES, PARTS (rings per polygon), RINGS (vertices per ring)
 * - multi, no polygon: GEOMETRIES, PARTS (vertices per line string)
 * - polygon, no multi: PARTS (rings per polygon), RINGS (vertices per ring)
 * - line strings only: PARTS (vertices per line string)
 * - points only: no length streams at all
 */
function encodeGeometryColumn(features: Feature[]): Uint8Array {
    const types = features.map((feature) => geometryTypeOf(feature.geometry));
    const hasMulti = types.some((type) => type > GEOMETRY_TYPE.POLYGON);
    const hasPolygon = types.some((type) => type === GEOMETRY_TYPE.POLYGON || type === GEOMETRY_TYPE.MULTIPOLYGON);
    const hasLine = types.some((type) => type === GEOMETRY_TYPE.LINESTRING || type === GEOMETRY_TYPE.MULTILINESTRING);

    const geometryLengths: number[] = [];
    const partLengths: number[] = [];
    const ringLengths: number[] = [];
    const vertices: number[] = [];

    for (const [index, feature] of features.entries()) {
        const type = types[index];
        // Every geometry is normalised to polygon -> ring -> position, so one walk covers them all.
        const polygons = toPolygons(feature.geometry);
        const polygonal = type === GEOMETRY_TYPE.POLYGON || type === GEOMETRY_TYPE.MULTIPOLYGON;
        const linear = type === GEOMETRY_TYPE.LINESTRING || type === GEOMETRY_TYPE.MULTILINESTRING;

        if (hasMulti && type > GEOMETRY_TYPE.POLYGON) {
            geometryLengths.push(polygons.length);
        }

        for (const rings of polygons) {
            if (hasMulti && hasPolygon) {
                // PARTS is read only for polygons; RINGS covers everything except point types.
                if (polygonal) partLengths.push(rings.length);
                if (polygonal || linear) for (const ring of rings) ringLengths.push(ring.length);
            } else if (hasMulti) {
                if (linear) partLengths.push(rings[0].length);
            } else if (hasPolygon) {
                // PARTS is read for anything above a line string, RINGS for polygons and lines.
                if (polygonal) partLengths.push(rings.length);
                if (polygonal || linear) for (const ring of rings) ringLengths.push(ring.length);
            } else if (hasLine && linear) {
                partLengths.push(rings[0].length);
            }

            for (const ring of rings) {
                for (const [x, y] of ring) vertices.push(x, y);
            }
        }
    }

    const streams = [
        createStream(PhysicalStreamType.LENGTH, encodeVarints(types), {
            technique: PhysicalLevelTechnique.VARINT,
            count: types.length,
        }),
    ];

    if (hasMulti) streams.push(lengthStream(LengthType.GEOMETRIES, geometryLengths));
    if (hasMulti || hasPolygon || hasLine) streams.push(lengthStream(LengthType.PARTS, partLengths));
    if (hasPolygon) streams.push(lengthStream(LengthType.RINGS, ringLengths));

    streams.push(
        createStream(PhysicalStreamType.DATA, encodeVarints(zigZag(vertices)), {
            logical: { dictionaryType: DictionaryType.VERTEX },
            technique: PhysicalLevelTechnique.VARINT,
            count: vertices.length,
        }),
    );

    return concatenateBuffers(encodeVarintValue(streams.length), ...streams);
}

function lengthStream(lengthType: LengthType, values: number[]): Uint8Array {
    return createStream(PhysicalStreamType.LENGTH, encodeVarints(values), {
        logical: { lengthType },
        technique: PhysicalLevelTechnique.VARINT,
        count: values.length,
    });
}

function geometryTypeOf(geometry: FeatureGeometry): number {
    switch (geometry.type) {
        case "Point":
            return GEOMETRY_TYPE.POINT;
        case "LineString":
            return GEOMETRY_TYPE.LINESTRING;
        case "Polygon":
            return GEOMETRY_TYPE.POLYGON;
        case "MultiPoint":
            return GEOMETRY_TYPE.MULTIPOINT;
        case "MultiLineString":
            return GEOMETRY_TYPE.MULTILINESTRING;
        case "MultiPolygon":
            return GEOMETRY_TYPE.MULTIPOLYGON;
    }
}

/** Normalises any geometry to polygon -> ring -> position, so one walk covers every type. */
function toPolygons(geometry: FeatureGeometry): Position[][][] {
    switch (geometry.type) {
        case "Point":
            return [[[geometry.coordinates]]];
        case "MultiPoint":
            return geometry.coordinates.map((position) => [[position]]);
        case "LineString":
            return [[geometry.coordinates]];
        case "MultiLineString":
            return geometry.coordinates.map((line) => [line]);
        case "Polygon":
            return [geometry.coordinates.map(openRing)];
        case "MultiPolygon":
            return geometry.coordinates.map((polygon) => polygon.map(openRing));
    }
}

/** Rings are stored without the repeated closing position; the decoder puts it back. */
function openRing(ring: Position[]): Position[] {
    if (ring.length < 2) return ring;
    const [firstX, firstY] = ring[0];
    const [lastX, lastY] = ring[ring.length - 1];
    return firstX === lastX && firstY === lastY ? ring.slice(0, -1) : ring;
}

function zigZag(values: number[]): Uint32Array {
    return encodeZigZagInt32(Int32Array.from(values));
}

function encodeVarints(values: number[] | Uint32Array): Uint8Array {
    return encodeVarintInt32(values instanceof Uint32Array ? values : Uint32Array.from(values));
}

function encodeVarintValue(value: number): Uint8Array {
    const buffer = new Uint8Array(5);
    const offset = new IntWrapper(0);
    encodeVarintInt32Value(value, buffer, offset);
    return buffer.slice(0, offset.get());
}
