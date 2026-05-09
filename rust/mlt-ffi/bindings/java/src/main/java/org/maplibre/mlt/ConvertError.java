package org.maplibre.mlt;

/**
 * Error type returned by FFI conversion functions.
 */
public enum ConvertError {
    /** Input bytes could not be parsed or decoded. */
    INVALID_INPUT(0),
    /** Encoding failed. */
    ENCODING_FAILED(1);

    final int code;

    ConvertError(int code) {
        this.code = code;
    }

    static ConvertError fromCode(int code) {
        return switch (code) {
            case 0 -> INVALID_INPUT;
            case 1 -> ENCODING_FAILED;
            default -> throw new IllegalArgumentException("Unknown ConvertError code: " + code);
        };
    }
}
