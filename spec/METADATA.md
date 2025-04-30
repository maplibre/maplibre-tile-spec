VectorType {
    FLAT = 0
    CONST = 1
    FREQUENCY = 2
    REE = 3
    DICTIONARY = 4
}
PhysicalStreamType {
    PRESENT = 0
    DATA = 1
    OFFSET = 2
    LENGTH = 3
}

LogicalLevelTechnique {
    NONE = 0
    DELTA = 1
    COMPONENTWISE_DELTA = 2
    RLE = 3
    MORTON = 4
    PDE = 5
}
PhysicalLevelTechnique {
    NONE = 0
    FAST_PFOR = 1
    VARINT = 2
    ALP = 3
}

DictionaryType {
    NONE = 0
    SINGLE = 1
    SHARED = 2
    VERTEX = 3
    MORTON = 4
    FSST = 5
}

OffsetType {
    VERTEX = 0
    INDEX = 1
    STRING = 2
    KEY = 3
}

LengthType {
    VAR_BINARY = 0
    GEOMETRIES = 1
    PARTS = 2
    RINGS = 3
    TRIANGLES = 4
    SYMBOL = 5
    DICTIONARY = 6
}

FeatureTableMetadata {
    version: u8
    id: varint
    featureTableBodySize: varint
    layerExtent: varint
    maxLayerExtent: varint
    numFeatures: varint
    fieldMetadata: FieldMetadata[]
}

FieldMetadata {
    numStreams: varint
    vectorType: VectorType as u8
    streamMetadata: StreamMetadata[]
}

StreamMetadata {
    physicalStreamType: PhysicalStreamType (bitfield: 4 bits)
    logicalStreamType: LogicalStreamType (bitfield: 4 bits)
    logicalLevelTechnique1 LogicalLevelTechnique (bitfield: 3 bits)
    logicalLevelTechnique2 LogicalLevelTechnique (bitfield: 3 bits)
    physicalLevelTechnique2 PhysicalLevelTechnique (bitfield: 2 bits)
    numValues: varint
    byteLength: varint
}

RleEncodedStreamMetadata : StreamMetadata {
    runs: varint
    numRleValues: varint
}

MortonEncodedStreamMetadata : StreamMetadata {
    numBits: u8
    coordinateShift: varint
}

LogicalStreamType {
    dictionaryType: DictionaryType?
    offsetType: OffsetType?
    lengthType: LengthType?
}
