export enum LogicalLevelTechnique {
    NONE = "NONE",
    DELTA = "DELTA",
    COMPONENTWISE_DELTA = "COMPONENTWISE_DELTA",
    RLE = "RLE",
    MORTON = "MORTON",
    // Pseudodecimal Encoding of floats -> only for the exponent integer part an additional logical level technique is used.
    // Both exponent and significant parts are encoded with the same physical level technique
    PDE = "PDE"
}
