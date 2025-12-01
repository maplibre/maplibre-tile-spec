import { type GeometryVector, type MortonSettings } from "../vector/geometry/geometryVector";
import { StreamMetadataDecoder } from "../metadata/tile/streamMetadataDecoder";
import type IntWrapper from "./intWrapper";
import {
    decodeConstIntStream,
    decodeIntStream,
    decodeLengthStreamToOffsetBuffer,
    getVectorType,
} from "./integerStreamDecoder";
import { VectorType } from "../vector/vectorType";
import { PhysicalStreamType } from "../metadata/tile/physicalStreamType";
import { LengthType } from "../metadata/tile/lengthType";
import { DictionaryType } from "../metadata/tile/dictionaryType";
import { type MortonEncodedStreamMetadata } from "../metadata/tile/mortonEncodedStreamMetadata";
import TopologyVector from "../vector/geometry/topologyVector";
import {
    ConstGeometryVector,
    createConstGeometryVector,
    createMortonEncodedConstGeometryVector,
} from "../vector/geometry/constGeometryVector";
import {
    createFlatGeometryVector,
    createFlatGeometryVectorMortonEncoded,
    FlatGeometryVector,
} from "../vector/geometry/flatGeometryVector";
import { OffsetType } from "../metadata/tile/offsetType";
import { ConstGpuVector, createConstGpuVector } from "../vector/geometry/constGpuVector";
import { type GpuVector } from "../vector/geometry/gpuVector";
import { createFlatGpuVector, FlatGpuVector } from "../vector/geometry/flatGpuVector";
import type GeometryScaling from "./geometryScaling";

// TODO: get rid of numFeatures parameter
export function decodeGeometryColumn(
    tile: Uint8Array,
    numStreams: number,
    offset: IntWrapper,
    numFeatures: number,
    scalingData?: GeometryScaling,
): GeometryVector | GpuVector {
    const geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
    const geometryTypesVectorType = getVectorType(geometryTypeMetadata, numFeatures, tile, offset);

    let geometryOffsets: Int32Array = null;
    let partOffsets: Int32Array = null;
    let ringOffsets: Int32Array = null;
    let vertexOffsets: Int32Array = null;
    let vertexBuffer: Int32Array = null;
    let mortonSettings: MortonSettings = null;
    //TODO: use geometryOffsets for that? -> but then tessellated polygons can't be used with normal polygons
    // in one FeatureTable?
    let triangleOffsets: Int32Array = null;
    let indexBuffer: Int32Array = null;

    if (geometryTypesVectorType === VectorType.CONST) {
        /* All geometries in the colum have the same geometry type */
        const geometryType = decodeConstIntStream(tile, offset, geometryTypeMetadata, false);

        for (let i = 0; i < numStreams - 1; i++) {
            const geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
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
                            vertexOffsets = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case OffsetType.INDEX:
                            indexBuffer = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                    }
                    break;
                }
                case PhysicalStreamType.DATA: {
                    if (DictionaryType.VERTEX === geometryStreamMetadata.logicalStreamType.dictionaryType) {
                        vertexBuffer = decodeIntStream(tile, offset, geometryStreamMetadata, true, scalingData);
                    } else {
                        const mortonMetadata = geometryStreamMetadata as MortonEncodedStreamMetadata;
                        mortonSettings = {
                            numBits: mortonMetadata.numBits(),
                            coordinateShift: mortonMetadata.coordinateShift(),
                        };
                        vertexBuffer = decodeIntStream(tile, offset, geometryStreamMetadata, false, scalingData);
                    }
                    break;
                }
            }
        }

        if (indexBuffer !== null) {
            if (geometryOffsets != null || partOffsets != null) {
                /* Case when the indices of a Polygon outline are encoded in the tile */
                const topologyVector = new TopologyVector(geometryOffsets, partOffsets, ringOffsets);
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

        return mortonSettings === null
            ? /* Currently only 2D coordinates (Vec2) are implemented in the encoder  */
              createConstGeometryVector(
                  numFeatures,
                  geometryType,
                  new TopologyVector(geometryOffsets, partOffsets, ringOffsets),
                  vertexOffsets,
                  vertexBuffer,
              )
            : createMortonEncodedConstGeometryVector(
                  numFeatures,
                  geometryType,
                  new TopologyVector(geometryOffsets, partOffsets, ringOffsets),
                  vertexOffsets,
                  vertexBuffer,
                  mortonSettings,
              );
    }

    /* Different geometry types are mixed in the geometry column */
    const geometryTypeVector = decodeIntStream(tile, offset, geometryTypeMetadata, false);

    for (let i = 0; i < numStreams - 1; i++) {
        const geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
        switch (geometryStreamMetadata.physicalStreamType) {
            case PhysicalStreamType.LENGTH:
                switch (geometryStreamMetadata.logicalStreamType.lengthType) {
                    case LengthType.GEOMETRIES:
                        geometryOffsets = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                    case LengthType.PARTS:
                        partOffsets = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                    case LengthType.RINGS:
                        ringOffsets = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                    case LengthType.TRIANGLES:
                        triangleOffsets = decodeLengthStreamToOffsetBuffer(tile, offset, geometryStreamMetadata);
                }
                break;
            case PhysicalStreamType.OFFSET:
                switch (geometryStreamMetadata.logicalStreamType.offsetType) {
                    case OffsetType.VERTEX:
                        vertexOffsets = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                    case OffsetType.INDEX:
                        indexBuffer = decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                }
                break;
            case PhysicalStreamType.DATA:
                if (DictionaryType.VERTEX === geometryStreamMetadata.logicalStreamType.dictionaryType) {
                    vertexBuffer = decodeIntStream(tile, offset, geometryStreamMetadata, true, scalingData);
                } else {
                    const mortonMetadata = geometryStreamMetadata as MortonEncodedStreamMetadata;
                    mortonSettings = {
                        numBits: mortonMetadata.numBits(),
                        coordinateShift: mortonMetadata.coordinateShift(),
                    };
                    vertexBuffer = decodeIntStream(tile, offset, geometryStreamMetadata, false, scalingData);
                }
                break;
        }
    }

    if (indexBuffer !== null && partOffsets === null) {
        /* Case when the indices of a Polygon outline are not encoded in the data so no
         *  topology data are present in the tile */
        return createFlatGpuVector(geometryTypeVector, triangleOffsets, indexBuffer, vertexBuffer);
    }

    // TODO: refactor the following instructions -> decode in one pass for performance reasons
    /* Calculate the offsets from the length buffer for util access */
    if (geometryOffsets !== null) {
        geometryOffsets = decodeRootLengthStream(geometryTypeVector, geometryOffsets, 2);
        if (partOffsets !== null && ringOffsets !== null) {
            partOffsets = decodeLevel1LengthStream(geometryTypeVector, geometryOffsets, partOffsets, false);
            ringOffsets = decodeLevel2LengthStream(geometryTypeVector, geometryOffsets, partOffsets, ringOffsets);
        } else if (partOffsets !== null) {
            partOffsets = decodeLevel1WithoutRingBufferLengthStream(geometryTypeVector, geometryOffsets, partOffsets);
        }
    } else if (partOffsets !== null && ringOffsets !== null) {
        partOffsets = decodeRootLengthStream(geometryTypeVector, partOffsets, 1);
        ringOffsets = decodeLevel1LengthStream(geometryTypeVector, partOffsets, ringOffsets, true);
    } else if (partOffsets !== null) {
        partOffsets = decodeRootLengthStream(geometryTypeVector, partOffsets, 0);
    }

    if (indexBuffer !== null) {
        /* Case when the indices of a Polygon outline are encoded in the tile */
        return createFlatGpuVector(
            geometryTypeVector,
            triangleOffsets,
            indexBuffer,
            vertexBuffer,
            new TopologyVector(geometryOffsets, partOffsets, ringOffsets),
        );
    }

    return mortonSettings === null /* Currently only 2D coordinates (Vec2) are implemented in the encoder  */
        ? createFlatGeometryVector(
              geometryTypeVector,
              new TopologyVector(geometryOffsets, partOffsets, ringOffsets),
              vertexOffsets,
              vertexBuffer,
          )
        : createFlatGeometryVectorMortonEncoded(
              geometryTypeVector,
              new TopologyVector(geometryOffsets, partOffsets, ringOffsets),
              vertexOffsets,
              vertexBuffer,
              mortonSettings,
          );
}

/*
 * Handle the parsing of the different topology length buffers separate not generic to reduce the
 * branching and improve the performance
 */
function decodeRootLengthStream(geometryTypes: Int32Array, rootLengthStream: Int32Array, bufferId: number): Int32Array {
    const rootBufferOffsets = new Int32Array(geometryTypes.length + 1);
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
    geometryTypes: Int32Array,
    rootOffsetBuffer: Int32Array,
    level1LengthBuffer: Int32Array,
    isLineStringPresent: boolean,
): Int32Array {
    const level1BufferOffsets = new Int32Array(rootOffsetBuffer[rootOffsetBuffer.length - 1] + 1);
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
    geometryTypes: Int32Array,
    rootOffsetBuffer: Int32Array,
    level1LengthBuffer: Int32Array,
): Int32Array {
    const level1BufferOffsets = new Int32Array(rootOffsetBuffer[rootOffsetBuffer.length - 1] + 1);
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
    geometryTypes: Int32Array,
    rootOffsetBuffer: Int32Array,
    level1OffsetBuffer: Int32Array,
    level2LengthBuffer: Int32Array,
): Int32Array {
    const level2BufferOffsets = new Int32Array(level1OffsetBuffer[level1OffsetBuffer.length - 1] + 1);
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
