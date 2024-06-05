export enum PhysicalLevelTechnique {
    NONE,
    /* Preferred option, tends to produce the best compression ratio and decoding performance.
    * But currently only limited to 32 bit integer. */
    FAST_PFOR,
    /* Can produce better results in combination with a heavyweight compression scheme like Gzip.
    *  Simple compression scheme where the decoder are easier to implement compared to FastPfor.*/
    VARINT,
    /* Adaptive Lossless floating-Point Compression */
    ALP
}

export enum LogicalLevelTechnique {
    NONE,
    DELTA,
    COMPONENTWISE_DELTA,
    RLE,
    MORTON,
    /* Pseudodecimal Encoding of floats -> only for the exponent integer part an additional logical level technique is used.
    *  Both exponent and significant parts are encoded with the same physical level technique */
    PDE
}

export enum OffsetType {
    VERTEX,
    INDEX,
    STRING,
    KEY
}

export enum DictionaryType {
    NONE,
    SINGLE,
    SHARED,
    VERTEX,
    MORTON,
    FSST
}

export enum LengthType {
    VAR_BINARY,
    GEOMETRIES,
    PARTS,
    RINGS,
    TRIANGLES,
    SYMBOL,
    DICTIONARY
}

export enum PhysicalStreamType {
    PRESENT,
    DATA,
    OFFSET,
    LENGTH
}

export interface LogicalStreamType {
    dictionaryType: DictionaryType;
    offsetType: OffsetType;
    lengthType: LengthType;
}

export interface StreamMetadata {
    physicalStreamType: PhysicalStreamType;
    logicalStreamType: LogicalStreamType;
    logicalLevelTechnique1: LogicalLevelTechnique;
    logicalLevelTechnique2: LogicalLevelTechnique;
    physicalLevelTechnique: PhysicalLevelTechnique;
    numValues: number;
    byteLength: number;
}

export function decodeStreamMetadata(buffer: Uint8Array, offset: number): [StreamMetadata, number] {
    let nextOffset = offset;
    const streamType = buffer[offset++];
    const physicalStreamType : PhysicalStreamType = Object.values(PhysicalStreamType)[streamType >> 4];

    let logicalStreamType, logicalLevelTechnique1, logicalLevelTechnique2, physicalLevelTechnique;

    return [{
        physicalStreamType,
        logicalStreamType,
        logicalLevelTechnique1,
        logicalLevelTechnique2,
        physicalLevelTechnique,
        numValues: 0,
        byteLength: 0
    }, nextOffset];
}
