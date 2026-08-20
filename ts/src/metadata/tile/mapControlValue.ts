/**
 * Tokens in the data stream of a nested property (MAP) column. Values below `COUNT` describe the
 * structure; anything else is an index into the combined dictionary, offset by `COUNT`. Booleans are
 * encoded directly as tokens rather than being added to a dictionary.
 */
export enum MapControlValue {
    FALSE = 0,
    TRUE = 1,
    /** A nested map follows: this token, the payload length including these two tokens, the payload. */
    START_MAP = 2,
    /** A list follows, laid out the same way as START_MAP. */
    START_LIST = 3,
    /** Number of reserved tokens, i.e. the first dictionary index. */
    COUNT = 4,
}
