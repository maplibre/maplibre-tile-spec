package org.maplibre.mlt;

/**
 * Exception thrown when an MLT conversion fails.
 */
public final class MltException extends Exception {
    private final ConvertError error;

    MltException(ConvertError error) {
        super(error.name());
        this.error = error;
    }

    public ConvertError getError() {
        return error;
    }
}
