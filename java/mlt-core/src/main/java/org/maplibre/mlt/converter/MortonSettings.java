package org.maplibre.mlt.converter;

public final class MortonSettings {
    public int numBits;
    public int coordinateShift;

    public MortonSettings(int numBits, int coordinateShift) {
        this.numBits = numBits;
        this.coordinateShift = coordinateShift;
    }
}
