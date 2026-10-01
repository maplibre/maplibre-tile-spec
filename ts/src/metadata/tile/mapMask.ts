/**
 * Bitmask written ahead of a nested property (MAP) column, marking which optional streams follow
 * the mandatory length stream. Only one of INT32/INT64 and one of UINT32/UINT64 is ever set: the
 * encoder picks the narrower width that fits every value.
 */
export enum MapMask {
    STRING = 1,
    INT32 = 1 << 1,
    UINT32 = 1 << 2,
    INT64 = 1 << 3,
    UINT64 = 1 << 4,
    FLOAT = 1 << 5,
    DOUBLE = 1 << 6,
    PRESENCE = 1 << 7,
}
