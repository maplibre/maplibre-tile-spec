import type { MltColumnLayer, MltGeometryLayout, MltIdColumn, MltPropertyColumn } from "./columns";
import FeatureTable from "./vector/featureTable";
import { type Column, ScalarType } from "./metadata/tileset/tilesetMetadata";
import IntWrapper from "./decoding/intWrapper";
import { decodeStreamMetadata, type RleEncodedStreamMetadata } from "./metadata/tile/streamMetadataDecoder";
import { VectorType } from "./vector/vectorType";
import { Int32FlatVector } from "./vector/flat/int32FlatVector";
import BitVector from "./vector/flat/bitVector";
import { BooleanFlatVector } from "./vector/flat/booleanFlatVector";
import { FloatFlatVector } from "./vector/flat/floatFlatVector";
import { ObjectFlatVector } from "./vector/flat/objectFlatVector";
import {
    decodeUnsignedConstInt32Stream,
    decodeUnsignedConstInt64Stream,
    decodeUnsignedInt64AsFloat64Stream,
    decodeUnsignedInt32Stream,
    decodeUnsignedInt64Stream,
    decodeSequenceInt32Stream,
    decodeSequenceInt64Stream,
    getVectorType,
} from "./decoding/integerStreamDecoder";
import { Int32SequenceVector } from "./vector/sequence/int32SequenceVector";
import { Int64FlatVector } from "./vector/flat/int64FlatVector";
import { Int64SequenceVector } from "./vector/sequence/int64SequenceVector";
import type { IdVector } from "./vector/idVector";
import { decodeVarintInt32 } from "./decoding/integerDecodingUtils";
import { decodeGeometryColumn } from "./decoding/geometryDecoder";
import { decodePropertyColumn } from "./decoding/propertyDecoder";
import { Int32ConstVector } from "./vector/constant/int32ConstVector";
import { Int64ConstVector } from "./vector/constant/int64ConstVector";
import type GeometryScaling from "./decoding/geometryScaling";
import { decodeBooleanRle } from "./decoding/decodingUtils";
import { DoubleFlatVector } from "./vector/flat/doubleFlatVector";
import { decodeEmbeddedTileSetMetadata } from "./metadata/tileset/embeddedTilesetMetadataDecoder";
import { hasStreamCount, isGeometryColumn, isLogicalIdColumn } from "./metadata/tileset/typeMap";
import type { StreamMetadata } from "./metadata/tile/streamMetadataDecoder";
import type { GeometryVector } from "./vector/geometry/geometryVector";
import { ConstGeometryVector } from "./vector/geometry/constGeometryVector";
import { FlatGeometryVector } from "./vector/geometry/flatGeometryVector";
import { createConstGpuVector } from "./vector/geometry/constGpuVector";
import { createFlatGpuVector } from "./vector/geometry/flatGpuVector";
import { GEOMETRY_TYPE } from "./vector/geometry/geometryType";
import type { TopologyVector } from "./vector/geometry/topologyVector";
import { VertexBufferType } from "./vector/geometry/vertexBufferType";
import Vector from "./vector/vector";
import { GpuVector } from "./vector/geometry/gpuVector";
import {
    allocF32,
    allocF64,
    allocI32,
    allocI64,
    allocU8,
    allocU32,
    allocU64,
    chargeBytes,
    resetBudget,
} from "./decoding/v2/binary";
import { vertexIndexAt } from "./decoding/v2/geometry";
import { decodeSingleLayer02 } from "./decoding/v2/layer";

/**
 * Decodes a tile with embedded metadata (Tag 0x01 and Tag 0x02 formats).
 * This is the primary decoder function for MLT tiles.
 *
 * @param tile The tile data to decode (will be decompressed if gzip-compressed)
 * @param geometryScaling Optional geometry scaling parameters
 * @param idWithinMaxSafeInteger If true, limits ID values to JavaScript safe integer range (53 bits)
 */
export default function decodeTile(
    tile: Uint8Array,
    geometryScaling?: GeometryScaling,
    idWithinMaxSafeInteger = true,
): FeatureTable[] {
    resetBudget();
    return decodeTileInternal(tile, geometryScaling, idWithinMaxSafeInteger);
}

function decodeTileInternal(
    tile: Uint8Array,
    geometryScaling?: GeometryScaling,
    idWithinMaxSafeInteger = true,
    unsignedCols?: Set<string>,
): FeatureTable[] {
    const offset = new IntWrapper(0);
    const featureTables: FeatureTable[] = [];

    while (offset.get() < tile.length) {
        const blockLength = decodeVarintInt32(tile, offset, 1)[0] >>> 0;
        const blockStart = offset.get();
        const blockEnd = blockStart + blockLength;
        if (blockEnd > tile.length) {
            throw new Error(`Block overruns tile: ${blockEnd} > ${tile.length}`);
        }

        const tag = decodeVarintInt32(tile, offset, 1)[0] >>> 0;
        if (tag === 2) {
            const layer = decodeSingleLayer02(tile.subarray(offset.get(), blockEnd), true, false, undefined, false);
            featureTables.push(v2LayerToFeatureTable(layer, geometryScaling, idWithinMaxSafeInteger));
            offset.set(blockEnd);
            continue;
        }
        if (tag !== 1) {
            // Skip unknown block types
            offset.set(blockEnd);
            continue;
        }

        const [metadata, extent] = decodeEmbeddedTileSetMetadata(tile, offset);
        const featureTableMetadata = metadata.featureTables[0];

        let idVector: IdVector | null = null;
        let geometryVector: GeometryVector | GpuVector | null = null;
        const propertyVectors: Vector[] = [];
        let numFeatures = 0;

        for (const columnMetadata of featureTableMetadata.columns) {
            const columnName = columnMetadata.name;

            if (isLogicalIdColumn(columnMetadata)) {
                let nullabilityBuffer = null;
                // Check column metadata nullable flag, not numStreams (ID columns don't have stream count)
                if (columnMetadata.nullable) {
                    const presentStreamMetadata = decodeStreamMetadata(tile, offset);
                    const streamDataStart = offset.get();
                    const values = decodeBooleanRle(
                        tile,
                        presentStreamMetadata.numValues,
                        presentStreamMetadata.byteLength,
                        offset,
                    );
                    offset.set(streamDataStart + presentStreamMetadata.byteLength);
                    nullabilityBuffer = new BitVector(values, presentStreamMetadata.numValues);
                }

                const idDataStreamMetadata = decodeStreamMetadata(tile, offset);
                // decompressedCount is the count WITHOUT nulls, but we may have nulls
                numFeatures = nullabilityBuffer ? nullabilityBuffer.size() : idDataStreamMetadata.decompressedCount;

                idVector = decodeIdColumn(
                    tile,
                    columnMetadata,
                    offset,
                    columnName,
                    idDataStreamMetadata,
                    nullabilityBuffer ?? numFeatures,
                    idWithinMaxSafeInteger,
                );
            } else if (isGeometryColumn(columnMetadata)) {
                const numStreams = decodeVarintInt32(tile, offset, 1)[0];

                // If no ID column, get numFeatures from geometry type stream metadata
                if (numFeatures === 0) {
                    const savedOffset = offset.get();
                    const geometryTypeMetadata = decodeStreamMetadata(tile, offset);
                    numFeatures = geometryTypeMetadata.decompressedCount;
                    offset.set(savedOffset); // Reset to re-read in decodeGeometryColumn
                }

                if (geometryScaling) {
                    geometryScaling.scale = geometryScaling.extent / extent;
                }

                geometryVector = decodeGeometryColumn(tile, numStreams, offset, numFeatures, geometryScaling);
            } else {
                const pt = columnMetadata.scalarType?.physicalType;
                if (unsignedCols && (pt === ScalarType.UINT_32 || pt === ScalarType.UINT_64)) {
                    unsignedCols.add(columnName);
                }
                const columnHasStreamCount = hasStreamCount(columnMetadata);
                const numStreams = columnHasStreamCount ? decodeVarintInt32(tile, offset, 1)[0] : 1;

                if (numStreams === 0) {
                    continue;
                }

                const propertyVector = decodePropertyColumn(
                    tile,
                    offset,
                    columnMetadata,
                    numStreams,
                    numFeatures,
                    undefined,
                );
                if (propertyVector) {
                    if (Array.isArray(propertyVector)) {
                        for (const property of propertyVector) {
                            propertyVectors.push(property);
                        }
                    } else {
                        propertyVectors.push(propertyVector);
                    }
                }
            }
        }

        const featureTable = new FeatureTable(
            featureTableMetadata.name,
            geometryVector,
            idVector,
            propertyVectors,
            extent,
        );
        featureTables.push(featureTable);
        offset.set(blockEnd);
    }

    return featureTables;
}

export function decodeV1LayerAsColumns(tile: Uint8Array, bodyStart: number, bodyEnd: number): MltColumnLayer {
    const bodyLen = bodyEnd - bodyStart;
    const hdr: number[] = [];
    let rem = bodyLen + 1;
    while (rem >= 0x80) {
        hdr.push((rem & 0x7f) | 0x80);
        rem >>>= 7;
    }
    hdr.push(rem, 1);
    const block = new Uint8Array(hdr.length + bodyLen);
    block.set(hdr, 0);
    block.set(tile.subarray(bodyStart, bodyEnd), hdr.length);

    const unsignedCols = new Set<string>();
    const table = decodeTileInternal(block, undefined, false, unsignedCols)[0];
    const n = table.numFeatures;
    const gv = table.geometryVector;
    const types = allocU8(n);
    for (let i = 0; i < n; i++) types[i] = gv.geometryType(i);

    let vertices: Int32Array;
    let triangleOffsets: Uint32Array | undefined;
    let indexBuffer: Uint32Array | undefined;
    let layout: MltGeometryLayout = "outlines";
    const top = gv.topologyVector;

    if (gv instanceof GpuVector) {
        vertices = allocI32(gv.vertexBuffer.length);
        vertices.set(gv.vertexBuffer);
        triangleOffsets = gv.triangleOffsets;
        indexBuffer = allocU32(gv.indexBuffer.length);
        indexBuffer.set(gv.indexBuffer);
        if (top) {
            layout = "tessPolygonsWithOutlines";
            let polyIdx = 0;
            for (let f = 0; f < n; f++) {
                if (types[f] === GEOMETRY_TYPE.POLYGON || types[f] === GEOMETRY_TYPE.MULTIPOLYGON) {
                    const base = vertexIndexAt(top, f);
                    for (let k = triangleOffsets[polyIdx] * 3; k < triangleOffsets[polyIdx + 1] * 3; k++) {
                        indexBuffer[k] += base;
                    }
                    polyIdx++;
                }
            }
        } else {
            layout = "tessPolygons";
        }
    } else if (gv.vertexOffsets && gv.vertexOffsets.length > 0) {
        const vc = gv.vertexOffsets.length;
        vertices = allocI32(vc * 2);
        for (let i = 0; i < vc; i++) {
            const [x, y] = gv.getVertex(i);
            vertices[i * 2] = x;
            vertices[i * 2 + 1] = y;
        }
    } else {
        vertices = allocI32(gv.vertexBuffer.length);
        vertices.set(gv.vertexBuffer);
    }

    const buildPresent = (vec: Vector): { present?: Uint8Array } => {
        let hasNull = false;
        const bmp = new Uint8Array((n + 7) >>> 3);
        for (let i = 0; i < n; i++) {
            if (vec.getValue(i) !== null) bmp[i >>> 3] |= 1 << (i & 7);
            else hasNull = true;
        }
        if (!hasNull) return {};
        chargeBytes(bmp.byteLength);
        return { present: bmp };
    };

    const fill = <A extends { [i: number]: number | bigint }>(
        alloc: (len: number) => A,
        fn: (i: number) => A[number],
    ): A => {
        const out = alloc(n);
        for (let i = 0; i < n; i++) out[i] = fn(i);
        return out;
    };

    let ids: MltIdColumn | undefined;
    if (table.idVector) {
        const idVec = table.idVector;
        const p = buildPresent(idVec);
        const is64 =
            idVec instanceof Int64FlatVector ||
            idVec instanceof Int64SequenceVector ||
            idVec instanceof Int64ConstVector;
        ids = is64
            ? { kind: "u64", ...p, values: fill(allocU64, (i) => BigInt.asUintN(64, BigInt(idVec.getValue(i) ?? 0))) }
            : { kind: "u32", ...p, values: fill(allocU32, (i) => Number(idVec.getValue(i) ?? 0) >>> 0) };
    }

    const properties: MltPropertyColumn[] = table.propertyVectors.map((vec): MltPropertyColumn => {
        const name = vec.name;
        const p = buildPresent(vec);
        if (vec instanceof BooleanFlatVector)
            return { name, kind: "bool", ...p, values: fill(allocU8, (i) => (vec.getValue(i) ? 1 : 0)) };
        if (vec instanceof FloatFlatVector)
            return { name, kind: "f32", ...p, values: fill(allocF32, (i) => vec.getValue(i) ?? 0) };
        if (vec instanceof DoubleFlatVector)
            return { name, kind: "f64", ...p, values: fill(allocF64, (i) => vec.getValue(i) ?? 0) };
        if (vec instanceof Int64FlatVector || vec instanceof Int64SequenceVector || vec instanceof Int64ConstVector) {
            return unsignedCols.has(name)
                ? { name, kind: "u64", ...p, values: fill(allocU64, (i) => BigInt.asUintN(64, vec.getValue(i) ?? 0n)) }
                : { name, kind: "i64", ...p, values: fill(allocI64, (i) => BigInt.asIntN(64, vec.getValue(i) ?? 0n)) };
        }
        if (vec instanceof Int32FlatVector || vec instanceof Int32SequenceVector || vec instanceof Int32ConstVector) {
            return unsignedCols.has(name)
                ? { name, kind: "u32", ...p, values: fill(allocU32, (i) => (vec.getValue(i) ?? 0) >>> 0) }
                : { name, kind: "i32", ...p, values: fill(allocI32, (i) => (vec.getValue(i) ?? 0) | 0) };
        }
        let chars = 0;
        const values = Array.from({ length: n }, (_, i) => {
            const s = String(vec.getValue(i) ?? "");
            chars += s.length;
            return s;
        });
        chargeBytes(chars * 2);
        return { name, kind: "str", ...p, values };
    });

    return {
        name: table.name,
        extent: table.extent,
        version: 1,
        featureCount: n,
        geometry: {
            types,
            layout,
            dimension: 2,
            vertices,
            ...(top?.geometryOffsets ? { geometryOffsets: top.geometryOffsets } : {}),
            ...(top?.partOffsets ? { partOffsets: top.partOffsets } : {}),
            ...(top?.ringOffsets ? { ringOffsets: top.ringOffsets } : {}),
            ...(triangleOffsets ? { triangleOffsets } : {}),
            ...(indexBuffer ? { indexBuffer } : {}),
        },
        ...(ids ? { ids } : {}),
        properties,
        vertexValues: [],
        triangleValues: [],
        mValues: [],
    };
}

function identityOffsets(count: number): Uint32Array {
    const out = new Uint32Array(count + 1);
    for (let i = 0; i <= count; i++) out[i] = i;
    return out;
}

function packBoolBytes(bools: Uint8Array): Uint8Array {
    const out = new Uint8Array((bools.length + 7) >>> 3);
    for (let i = 0; i < bools.length; i++) if (bools[i] !== 0) out[i >>> 3] |= 1 << (i & 7);
    return out;
}

const utf8Enc = new TextEncoder();
const pbfString = (s: string): string =>
    s.charCodeAt(0) === 0xfeff && utf8Enc.encode(s).length >= 12 ? s.slice(1) : s;

class DictStringVector extends Vector<Uint32Array, string> {
    constructor(
        name: string,
        codes: Uint32Array,
        private readonly dictionary: readonly string[],
        sizeOrNull: number | BitVector,
    ) {
        super(name, codes, sizeOrNull);
    }

    protected getValueFromBuffer(index: number): string {
        return pbfString(this.dictionary[this.dataBuffer[index]] ?? "");
    }
}

function v2PropertyToVector(col: MltPropertyColumn, numFeatures: number): Vector {
    const nullability = col.present ? new BitVector(col.present, numFeatures) : undefined;
    const sizeOrNull = nullability ?? numFeatures;
    if ("codes" in col) {
        return new DictStringVector(col.name, col.codes, col.dictionary, sizeOrNull);
    }
    switch (col.kind) {
        case "i32":
        case "u32":
            return new Int32FlatVector(col.name, col.values, sizeOrNull);
        case "i8":
        case "u8":
            return new Int32FlatVector(col.name, Int32Array.from(col.values), sizeOrNull);
        case "i64":
        case "u64":
            return new Int64FlatVector(col.name, col.values, sizeOrNull);
        case "f32":
            return new FloatFlatVector(col.name, col.values, sizeOrNull);
        case "f64":
            return new DoubleFlatVector(col.name, col.values, sizeOrNull);
        case "bool":
            return new BooleanFlatVector(col.name, new BitVector(packBoolBytes(col.values), numFeatures), sizeOrNull);
        case "str":
            return new ObjectFlatVector(col.name, col.values.map(pbfString), nullability);
    }
}

function v2LayerToFeatureTable(
    layer: MltColumnLayer,
    geometryScaling: GeometryScaling | undefined,
    idWithinMaxSafeInteger: boolean,
): FeatureTable {
    const numFeatures = layer.featureCount;
    const geom = layer.geometry;
    const stride = geom.dimension;
    let vertices = geom.vertices;

    if (geometryScaling) {
        const scale = geometryScaling.extent / layer.extent;
        geometryScaling.scale = scale;
        const { min, max } = geometryScaling;
        const scaled = new Int32Array(vertices.length);
        for (let i = 0; i < vertices.length; i += stride) {
            scaled[i] = Math.min(Math.max(Math.round(vertices[i] * scale), min), max);
            scaled[i + 1] = Math.min(Math.max(Math.round(vertices[i + 1] * scale), min), max);
            if (stride === 3) scaled[i + 2] = vertices[i + 2];
        }
        vertices = scaled;
    }

    const vbType = stride === 3 ? VertexBufferType.VEC_3 : VertexBufferType.VEC_2;
    const types32 = Uint32Array.from(geom.types);
    let isConst = numFeatures > 0;
    let needsGeomOffsets = false;
    let needsPartOffsets = geom.ringOffsets !== undefined;

    for (let i = 0; i < numFeatures; i++) {
        const t = types32[i];
        if (t !== types32[0]) isConst = false;
        if (
            t === GEOMETRY_TYPE.MULTIPOINT ||
            t === GEOMETRY_TYPE.MULTILINESTRING ||
            t === GEOMETRY_TYPE.MULTIPOLYGON ||
            t === GEOMETRY_TYPE.MESH
        ) {
            needsGeomOffsets = true;
        }
        if (
            t === GEOMETRY_TYPE.LINESTRING ||
            t === GEOMETRY_TYPE.MULTILINESTRING ||
            t === GEOMETRY_TYPE.POLYGON ||
            t === GEOMETRY_TYPE.MULTIPOLYGON
        ) {
            needsPartOffsets = true;
        }
    }

    let topology: TopologyVector | undefined;
    if (geom.layout !== "tessPolygons") {
        const geometryOffsets = geom.geometryOffsets ?? (needsGeomOffsets ? identityOffsets(numFeatures) : undefined);
        const numGeoms = geometryOffsets ? geometryOffsets[numFeatures] : numFeatures;
        topology = {
            geometryOffsets,
            partOffsets: geom.partOffsets ?? (needsPartOffsets ? identityOffsets(numGeoms) : undefined),
            ringOffsets: geom.ringOffsets,
        };
    }

    const geometryVector: GeometryVector | GpuVector =
        geom.indexBuffer && geom.triangleOffsets
            ? isConst
                ? createConstGpuVector(
                      numFeatures,
                      types32[0],
                      geom.triangleOffsets,
                      geom.indexBuffer,
                      vertices,
                      topology,
                      vbType,
                  )
                : createFlatGpuVector(types32, geom.triangleOffsets, geom.indexBuffer, vertices, topology, vbType)
            : isConst
              ? new ConstGeometryVector(numFeatures, types32[0], vbType, topology ?? {}, undefined, vertices)
              : new FlatGeometryVector(vbType, types32, topology ?? {}, undefined, vertices);

    let idVector: IdVector | undefined;
    if (layer.ids) {
        const nullability = layer.ids.present ? new BitVector(layer.ids.present, numFeatures) : numFeatures;
        idVector =
            layer.ids.kind === "u32"
                ? new Int32FlatVector("_id", layer.ids.values, nullability)
                : idWithinMaxSafeInteger
                  ? new DoubleFlatVector("_id", Float64Array.from(layer.ids.values, Number), nullability)
                  : new Int64FlatVector("_id", layer.ids.values, nullability);
    }

    const propertyVectors: Vector[] = layer.properties.map((col) => v2PropertyToVector(col, numFeatures));
    for (const ncol of layer.nested ?? []) {
        const nullability = ncol.present ? new BitVector(ncol.present, numFeatures) : undefined;
        propertyVectors.push(new ObjectFlatVector(ncol.name, [...ncol.values], nullability));
    }

    return new FeatureTable(layer.name, geometryVector, idVector, propertyVectors, layer.extent, {
        version: 2,
        zStep: geom.zStep,
        heightMode: geom.heightMode,
        vertexValues: layer.vertexValues,
        triangleValues: layer.triangleValues,
        mValues: layer.mValues,
        columnGeometry: geom,
    });
}

function decodeIdColumn(
    tile: Uint8Array,
    columnMetadata: Column,
    offset: IntWrapper,
    columnName: string,
    idDataStreamMetadata: StreamMetadata,
    sizeOrNullabilityBuffer: number | BitVector,
    idWithinMaxSafeInteger = false,
): IdVector {
    const idDataType = columnMetadata.scalarType?.longID ? ScalarType.UINT_64 : ScalarType.UINT_32;
    const nullabilityBuffer = typeof sizeOrNullabilityBuffer === "number" ? undefined : sizeOrNullabilityBuffer;

    const vectorType = getVectorType(
        idDataStreamMetadata,
        sizeOrNullabilityBuffer,
        tile,
        offset,
        idDataType === ScalarType.UINT_64 ? "int64" : "int32",
    );
    if (idDataType === ScalarType.UINT_32) {
        switch (vectorType) {
            case VectorType.FLAT: {
                const id = decodeUnsignedInt32Stream(tile, offset, idDataStreamMetadata, undefined, nullabilityBuffer);
                return new Int32FlatVector(columnName, id, sizeOrNullabilityBuffer);
            }
            case VectorType.SEQUENCE: {
                const id = decodeSequenceInt32Stream(tile, offset, idDataStreamMetadata);
                return new Int32SequenceVector(
                    columnName,
                    id[0],
                    id[1],
                    (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
                    false,
                );
            }
            case VectorType.CONST: {
                const id = decodeUnsignedConstInt32Stream(tile, offset, idDataStreamMetadata);
                return new Int32ConstVector(columnName, id, sizeOrNullabilityBuffer, false);
            }
        }
    }
    switch (vectorType) {
        case VectorType.FLAT: {
            if (idWithinMaxSafeInteger) {
                const id = decodeUnsignedInt64AsFloat64Stream(tile, offset, idDataStreamMetadata, nullabilityBuffer);
                return new DoubleFlatVector(columnName, id, sizeOrNullabilityBuffer);
            }
            const id = decodeUnsignedInt64Stream(tile, offset, idDataStreamMetadata, nullabilityBuffer);
            return new Int64FlatVector(columnName, id, sizeOrNullabilityBuffer);
        }
        case VectorType.SEQUENCE: {
            const id = decodeSequenceInt64Stream(tile, offset, idDataStreamMetadata);
            return new Int64SequenceVector(
                columnName,
                id[0],
                id[1],
                (idDataStreamMetadata as RleEncodedStreamMetadata).numRleValues,
                false,
            );
        }
        case VectorType.CONST: {
            const id = decodeUnsignedConstInt64Stream(tile, offset, idDataStreamMetadata);
            return new Int64ConstVector(columnName, id, sizeOrNullabilityBuffer, false);
        }
    }

    throw new Error("Vector type not supported for id column.");
}
