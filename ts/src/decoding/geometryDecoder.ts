import { decodeStreamMetadata, type MortonEncodedStreamMetadata } from "../metadata/tile/streamMetadataDecoder";
import type IntWrapper from "./intWrapper";
import {
    decodeSignedInt32Stream,
    decodeLengthStreamToOffsetBuffer,
    decodeUnsignedConstInt32Stream,
    decodeUnsignedInt32Stream,
    getVectorType,
} from "./integerStreamDecoder";
import { VectorType } from "../vector/vectorType";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LengthType } from "../metadata/tile/lengthType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import {
    createConstGeometryVector,
    createMortonEncodedConstGeometryVector,
} from "../vector/geometry/constGeometryVector";
import { createFlatGeometryVector, createFlatGeometryVectorMortonEncoded } from "../vector/geometry/flatGeometryVector";
import { OffsetType } from "../metadata/tile/offsetType";
import { createConstGpuVector } from "../vector/geometry/constGpuVector";
import { createFlatGpuVector } from "../vector/geometry/flatGpuVector";
import type { GeometryVector, MortonSettings } from "../vector/geometry/geometryVector";
import type { GpuVector } from "../vector/geometry/gpuVector";
import type GeometryScaling from "./geometryScaling";

// TODO: get rid of numFeatures parameter
export function decodeGeometryColumn(
    tile: Uint8Array,
    numStreams: number,
    offset: IntWrapper,
    numFeatures: number,
    scalingData?: GeometryScaling,
): GeometryVector | GpuVector {
    const geometryTypeMetadata = decodeStreamMetadata(tile, offset);
    const geometryTypesVectorType = getVectorType(geometryTypeMetadata, numFeatures, tile, offset);

    let vertexOffsets: Uint32Array | undefined;
    let vertexBuffer: Int32Array | Uint32Array | undefined;
    let mortonSettings: MortonSettings | undefined;
    let indexBuffer: Uint32Array | undefined;

    if (geometryTypesVectorType === VectorType.CONST) {
        /* All geometries in the column have the same geometry type */
        const geometryType = decodeUnsignedConstInt32Stream(tile, offset, geometryTypeMetadata);

        // Variables for const geometry path (directly decoded as offsets)
        let geometryOffsets: Uint32Array | undefined;
        let partOffsets: Uint32Array | undefined;
        let ringOffsets: Uint32Array | undefined;
        //TODO: use geometryOffsets for that? -> but then tessellated polygons can't be used with normal polygons
        // in one FeatureTable?
        let triangleOffsets: Uint32Array | undefined;

        for (let i = 0; i < numStreams - 1; i++) {
            const geometryStreamMetadata = decodeStreamMetadata(tile, offset);
            switch (geometryStreamMetadata.physicalStreamType) {
                case PhysicalStreamType.LENGTH:
                    switch (geometryStreamMetadata.logicalStreamType.lengthType) {
                        case LengthType.GEOMETRIES:
                            geometryOffsets = decodeLengthStreamToOffsetBuffer(tile, offset, geometryStreamMetadata);
                            break;
                        case LengthType.PARTS:
                            partOffsets = decodeLengthStreamToOffsetBuffer(tile, offset, geometryStreamMetadata);
                            break;
                        case LengthType.RINGS:
                            ringOffsets = decodeLengthStreamToOffsetBuffer(tile, offset, geometryStreamMetadata);
                            break;
                        case LengthType.TRIANGLES:
                            triangleOffsets = decodeLengthStreamToOffsetBuffer(tile, offset, geometryStreamMetadata);
                    }
                    break;
                case PhysicalStreamType.OFFSET: {
                    switch (geometryStreamMetadata.logicalStreamType.offsetType) {
                        case OffsetType.VERTEX:
                            vertexOffsets = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                            break;
                        case OffsetType.INDEX:
                            indexBuffer = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                            break;
                    }
                    break;
                }
                case PhysicalStreamType.DATA: {
                    if (DictionaryType.VERTEX === geometryStreamMetadata.logicalStreamType.dictionaryType) {
                        vertexBuffer = decodeSignedInt32Stream(tile, offset, geometryStreamMetadata, scalingData);
                    } else {
                        const mortonMetadata = geometryStreamMetadata as MortonEncodedStreamMetadata;
                        mortonSettings = {
                            numBits: mortonMetadata.numBits,
                            coordinateShift: mortonMetadata.coordinateShift,
                        };
                        vertexBuffer = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata, scalingData);
                    }
                    break;
                }
            }
        }

        if (indexBuffer) {
            if (geometryOffsets !== undefined || partOffsets !== undefined) {
                /* Case when the indices of a Polygon outline are encoded in the tile */
                const topologyVector = { geometryOffsets, partOffsets, ringOffsets };
                return createConstGpuVector(
                    numFeatures,
                    geometryType,
                    triangleOffsets,
                    indexBuffer,
                    vertexBuffer,
                    topologyVector,
                );
            }

            /* Case when the no Polygon outlines are encoded in the tile */
            return createConstGpuVector(numFeatures, geometryType, triangleOffsets, indexBuffer, vertexBuffer);
        }

        return mortonSettings === undefined
            ? /* Currently only 2D coordinates (Vec2) are implemented in the encoder  */
              createConstGeometryVector(
                  numFeatures,
                  geometryType,
                  { geometryOffsets, partOffsets, ringOffsets },
                  vertexOffsets,
                  vertexBuffer,
              )
            : createMortonEncodedConstGeometryVector(
                  numFeatures,
                  geometryType,
                  { geometryOffsets, partOffsets, ringOffsets },
                  vertexOffsets,
                  vertexBuffer,
                  mortonSettings,
              );
    }

    /* Different geometry types are mixed in the geometry column */
    const geometryTypeVector = decodeUnsignedInt32Stream(tile, offset, geometryTypeMetadata);

    // Variables for flat geometry path (decoded as lengths, then converted to offsets)
    let geometryLengths: Uint32Array | undefined;
    let partLengths: Uint32Array | undefined;
    let ringLengths: Uint32Array | undefined;
    //TODO: use geometryOffsets for that? -> but then tessellated polygons can't be used with normal polygons
    // in one FeatureTable?
    let triangleOffsets: Uint32Array | undefined;

    for (let i = 0; i < numStreams - 1; i++) {
        const geometryStreamMetadata = decodeStreamMetadata(tile, offset);
        switch (geometryStreamMetadata.physicalStreamType) {
            case PhysicalStreamType.LENGTH:
                switch (geometryStreamMetadata.logicalStreamType.lengthType) {
                    case LengthType.GEOMETRIES:
                        geometryLengths = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                        break;
                    case LengthType.PARTS:
                        partLengths = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                        break;
                    case LengthType.RINGS:
                        ringLengths = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                        break;
                    case LengthType.TRIANGLES:
                        triangleOffsets = decodeLengthStreamToOffsetBuffer(tile, offset, geometryStreamMetadata);
                }
                break;
            case PhysicalStreamType.OFFSET:
                switch (geometryStreamMetadata.logicalStreamType.offsetType) {
                    case OffsetType.VERTEX:
                        vertexOffsets = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                        break;
                    case OffsetType.INDEX:
                        indexBuffer = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata);
                        break;
                }
                break;
            case PhysicalStreamType.DATA:
                if (DictionaryType.VERTEX === geometryStreamMetadata.logicalStreamType.dictionaryType) {
                    vertexBuffer = decodeSignedInt32Stream(tile, offset, geometryStreamMetadata, scalingData);
                } else {
                    const mortonMetadata = geometryStreamMetadata as MortonEncodedStreamMetadata;
                    mortonSettings = {
                        numBits: mortonMetadata.numBits,
                        coordinateShift: mortonMetadata.coordinateShift,
                    };
                    vertexBuffer = decodeUnsignedInt32Stream(tile, offset, geometryStreamMetadata, scalingData);
                }
                break;
        }
    }

    // TODO: refactor the following instructions -> decode in one pass for performance reasons
    /* Calculate the offsets from the length buffer for util access */
    let geometryOffsets: Uint32Array | undefined;
    let partOffsets: Uint32Array | undefined;
    let ringOffsets: Uint32Array | undefined;

    if (geometryLengths) {
        geometryOffsets = decodeRootLengthStream(geometryTypeVector, geometryLengths, 2);
        if (partLengths && ringLengths) {
            partOffsets = decodeLevel1LengthStream(geometryTypeVector, geometryOffsets, partLengths, false);
            ringOffsets = decodeLevel2LengthStream(geometryTypeVector, geometryOffsets, partOffsets, ringLengths);
        } else if (partLengths) {
            partOffsets = decodeLevel1WithoutRingBufferLengthStream(geometryTypeVector, geometryOffsets, partLengths);
        }
    } else if (partLengths && ringLengths) {
        partOffsets = decodeRootLengthStream(geometryTypeVector, partLengths, 1);
        ringOffsets = decodeLevel1LengthStream(geometryTypeVector, partOffsets, ringLengths, true);
    } else if (partLengths) {
        partOffsets = decodeRootLengthStream(geometryTypeVector, partLengths, 0);
    }

    if (indexBuffer && !partOffsets) {
        /* Case when the indices of a Polygon outline are not encoded in the data so no
         *  topology data are present in the tile */
        return createFlatGpuVector(geometryTypeVector, triangleOffsets, indexBuffer, vertexBuffer);
    }

    if (indexBuffer) {
        /* Case when the indices of a Polygon outline are encoded in the tile */
        return createFlatGpuVector(geometryTypeVector, triangleOffsets, indexBuffer, vertexBuffer, {
            geometryOffsets,
            partOffsets,
            ringOffsets,
        });
    }

    return mortonSettings === undefined /* Currently only 2D coordinates (Vec2) are implemented in the encoder  */
        ? createFlatGeometryVector(
              geometryTypeVector,
              { geometryOffsets, partOffsets, ringOffsets },
              vertexOffsets,
              vertexBuffer,
          )
        : createFlatGeometryVectorMortonEncoded(
              geometryTypeVector,
              { geometryOffsets, partOffsets, ringOffsets },
              vertexOffsets,
              vertexBuffer,
              mortonSettings,
          );
}

/*
 * Handle the parsing of the different topology length buffers separate not generic to reduce the
 * branching and improve the performance
 */
function decodeRootLengthStream(
    geometryTypes: Uint32Array,
    rootLengthStream: Uint32Array,
    bufferId: number,
): Uint32Array {
    const rootBufferOffsets = new Uint32Array(geometryTypes.length + 1);
    let previousOffset = 0;
    rootBufferOffsets[0] = previousOffset;
    let rootLengthCounter = 0;
    for (let i = 0; i < geometryTypes.length; i++) {
        /* Test if the geometry has and entry in the root buffer
         * BufferId: 2 GeometryOffsets -> MultiPolygon, MultiLineString, MultiPoint
         * BufferId: 1 PartOffsets -> Polygon
         * BufferId: 0 PartOffsets, RingOffsets -> LineString
         * */
        previousOffset = rootBufferOffsets[i + 1] =
            previousOffset + (geometryTypes[i] > bufferId ? rootLengthStream[rootLengthCounter++] : 1);
    }

    return rootBufferOffsets;
}

function decodeLevel1LengthStream(
    geometryTypes: Uint32Array,
    rootOffsetBuffer: Uint32Array,
    level1LengthBuffer: Uint32Array,
    isLineStringPresent: boolean,
): Uint32Array {
    const level1BufferOffsets = new Uint32Array(rootOffsetBuffer[rootOffsetBuffer.length - 1] + 1);
    let previousOffset = 0;
    level1BufferOffsets[0] = previousOffset;
    let level1BufferCounter = 1;
    let level1LengthBufferCounter = 0;
    for (let i = 0; i < geometryTypes.length; i++) {
        const geometryType = geometryTypes[i];
        const numGeometries = rootOffsetBuffer[i + 1] - rootOffsetBuffer[i];
        if (
            geometryType === 5 ||
            geometryType === 2 ||
            (isLineStringPresent && (geometryType === 4 || geometryType === 1))
        ) {
            /* For MultiPolygon, Polygon and in some cases for MultiLineString and LineString
             * a value in the level1LengthBuffer exists */
            for (let j = 0; j < numGeometries; j++) {
                previousOffset = level1BufferOffsets[level1BufferCounter++] =
                    previousOffset + level1LengthBuffer[level1LengthBufferCounter++];
            }
        } else {
            /* For MultiPoint and Point and in some cases for MultiLineString and LineString no value in the
             * level1LengthBuffer exists */
            for (let j = 0; j < numGeometries; j++) {
                level1BufferOffsets[level1BufferCounter++] = ++previousOffset;
            }
        }
    }

    return level1BufferOffsets;
}

/*
 * Case where no ring buffer exists so no MultiPolygon or Polygon geometry is part of the buffer
 */
function decodeLevel1WithoutRingBufferLengthStream(
    geometryTypes: Uint32Array,
    rootOffsetBuffer: Uint32Array,
    level1LengthBuffer: Uint32Array,
): Uint32Array {
    const level1BufferOffsets = new Uint32Array(rootOffsetBuffer[rootOffsetBuffer.length - 1] + 1);
    let previousOffset = 0;
    level1BufferOffsets[0] = previousOffset;
    let level1OffsetBufferCounter = 1;
    let level1LengthCounter = 0;
    for (let i = 0; i < geometryTypes.length; i++) {
        const geometryType = geometryTypes[i];
        const numGeometries = rootOffsetBuffer[i + 1] - rootOffsetBuffer[i];
        if (geometryType === 4 || geometryType === 1) {
            /* For MultiLineString and LineString a value in the level1LengthBuffer exists */
            for (let j = 0; j < numGeometries; j++) {
                previousOffset = level1BufferOffsets[level1OffsetBufferCounter++] =
                    previousOffset + level1LengthBuffer[level1LengthCounter++];
            }
        } else {
            /* For MultiPoint and Point no value in level1LengthBuffer exists */
            for (let j = 0; j < numGeometries; j++) {
                level1BufferOffsets[level1OffsetBufferCounter++] = ++previousOffset;
            }
        }
    }

    return level1BufferOffsets;
}

function decodeLevel2LengthStream(
    geometryTypes: Uint32Array,
    rootOffsetBuffer: Uint32Array,
    level1OffsetBuffer: Uint32Array,
    level2LengthBuffer: Uint32Array,
): Uint32Array {
    const level2BufferOffsets = new Uint32Array(level1OffsetBuffer[level1OffsetBuffer.length - 1] + 1);
    let previousOffset = 0;
    level2BufferOffsets[0] = previousOffset;
    let level1OffsetBufferCounter = 1;
    let level2OffsetBufferCounter = 1;
    let level2LengthBufferCounter = 0;
    for (let i = 0; i < geometryTypes.length; i++) {
        const geometryType = geometryTypes[i];
        const numGeometries = rootOffsetBuffer[i + 1] - rootOffsetBuffer[i];
        if (geometryType !== 0 && geometryType !== 3) {
            /* For MultiPolygon, MultiLineString, Polygon and LineString a value in level2LengthBuffer
             * exists */
            for (let j = 0; j < numGeometries; j++) {
                const numParts =
                    level1OffsetBuffer[level1OffsetBufferCounter] - level1OffsetBuffer[level1OffsetBufferCounter - 1];
                level1OffsetBufferCounter++;
                for (let k = 0; k < numParts; k++) {
                    previousOffset = level2BufferOffsets[level2OffsetBufferCounter++] =
                        previousOffset + level2LengthBuffer[level2LengthBufferCounter++];
                }
            }
        } else {
            /* For MultiPoint and Point no value in level2LengthBuffer exists */
            for (let j = 0; j < numGeometries; j++) {
                level2BufferOffsets[level2OffsetBufferCounter++] = ++previousOffset;
                level1OffsetBufferCounter++;
            }
        }
    }

    return level2BufferOffsets;
}
