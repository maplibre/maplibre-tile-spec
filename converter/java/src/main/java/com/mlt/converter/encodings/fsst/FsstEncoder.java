package com.mlt.converter.encodings.fsst;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.FileSystems;
import java.io.ByteArrayOutputStream;

public class FsstEncoder {

    static {
        String modulePath = FileSystems.getDefault()
                    .getPath("build/FsstWrapper.so")
                    .normalize().toAbsolutePath().toString();
        try {
            System.load(modulePath);
        } catch (UnsatisfiedLinkError e) {
            System.out.println("Error: " + e.getMessage() + " - " + modulePath);
        }
    }

    public static SymbolTable encode(byte[] data){
        return FsstEncoder.compress(data);
    }

    public static byte[] decode(byte[] symbols, int[] symbolLengths, byte[] compressedData) throws IOException {
        ByteArrayOutputStream decodedData = new ByteArrayOutputStream();
        ByteBuffer symbolsBuffer = ByteBuffer.wrap(symbols).order(ByteOrder.BIG_ENDIAN);

        int[] symbolOffsets = new int[symbolLengths.length];
        symbolOffsets[0] = 0;
        for (int i = 1; i < symbolLengths.length; i++)  {
            symbolOffsets[i] = symbolOffsets[i - 1] + symbolLengths[i - 1];
        }

        for (int i = 0; i < compressedData.length; i++) {
            // In Java a byte[] is signed [-128 to 127], whereas in C++ it is unsigned [0 to 255]
            // So we do a bit shifting operation to convert the values into unsigned values for easier handling
            int symbolIndex = compressedData[i] & 0xFF;
            // 255 is our escape byte -> take the next symbol as it is
            if (symbolIndex == 255) {
                decodedData.write(compressedData[++i] & 0xFF);
            } else if (symbolIndex < symbolLengths.length) {
                int symbolLength = symbolLengths[symbolIndex];
                int symbolOffset = symbolOffsets[symbolIndex];
                byte[] symbolBytes = new byte[symbolLength];
                for (int j = 0; j < symbolLength; j++) {
                    symbolBytes[j] = symbolsBuffer.get(symbolOffset + j);
                }
                decodedData.write(symbolBytes);
            }
        }

        return decodedData.toByteArray();
    }

    public static native SymbolTable compress(byte[] inputBytes);
}