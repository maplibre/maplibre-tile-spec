package com.mlt.decoder.vectorized;

import com.mlt.metadata.stream.LogicalLevelTechnique;
import com.mlt.metadata.stream.RleEncodedStreamMetadata;
import com.mlt.metadata.stream.StreamMetadata;
import com.mlt.vector.BitVector;
import com.mlt.vector.VectorType;
import me.lemire.integercompression.*;
import org.apache.orc.impl.BufferChunk;
import org.apache.orc.impl.InStream;
import org.apache.orc.impl.RunLengthByteReader;

import java.io.IOException;
import java.nio.*;
import java.util.BitSet;

public class VectorizedDecodingUtils {

    /*public static byte[] encodeBooleanRle(BitSet bitSet, int numValues) throws IOException {
        var byteValues = bitSet.toByteArray();
        var numMissingBytes = (int)Math.ceil(numValues /8d) - (int)Math.ceil(bitSet.length() / 8d);
        if(numMissingBytes != 0){
            var paddingBytes = new byte[numMissingBytes];
            Arrays.fill(paddingBytes, (byte)0);
            byteValues = ArrayUtils.addAll(byteValues, paddingBytes);
        }

        var valueBuffer = new ArrayList<Byte>();
        var runsBuffer = new ArrayList<Integer>();
        byte previousValue = 0;
        var runs = 0;
        for(var i = 0; i < byteValues.length; i++){
            var value = byteValues[i];
            if(previousValue != value && i != 0){
                valueBuffer.add(previousValue);
                runsBuffer.add(runs);
                runs = 0;
            }

            runs++;
            previousValue = value;
        }
        valueBuffer.add(byteValues[byteValues.length -1]);
        runsBuffer.add(runs);

        var fastPforEncodedRuns = EncodingUtils.encodeFastPfor128(runsBuffer.stream().mapToInt(i -> i).toArray(), false, false);
        var varintEncodedRuns = EncodingUtils.encodeVarints(runsBuffer.stream().mapToLong(i -> i).toArray(), false, false);
        var encodedRuns = fastPforEncodedRuns.length < varintEncodedRuns.length? fastPforEncodedRuns: varintEncodedRuns;
        System.out.println("FastPfor encoded runs: " + fastPforEncodedRuns.length + " Varint encoded runs: "
                + varintEncodedRuns.length);

        System.out.println("Num Values: " + numValues + " Num Runs: " + runsBuffer.size() + " -----------------------------------------------");
        return ArrayUtils.addAll(encodedRuns, Bytes.toArray(valueBuffer));
    }*/

    public static byte[] decodeByteRle(byte[] buffer, int numBytes, int byteSize, IntWrapper pos) throws IOException {
        var inStream = InStream.create
                ("test", new BufferChunk(ByteBuffer.wrap(buffer), 0), pos.get(), buffer.length);
        var reader =
                new RunLengthByteReader(inStream);

        var values = new byte[numBytes];
        for(var i = 0; i < numBytes; i++){
            values[i] = reader.next();
        }

        pos.add(byteSize);
        return values;
    }

    public static ByteBuffer decodeBooleanRle(byte[] buffer, int numBooleans, IntWrapper pos) {
        var numBytes = (int)Math.ceil(numBooleans / 8d);
        return decodeByteRle(buffer, numBytes, pos);
    }

    public static ByteBuffer decodeNullableBooleanRle(byte[] buffer, int numBooleans, IntWrapper pos,
                                                      BitVector nullabilityBuffer) {
        //TODO: refactor quick and dirty solution -> use vectorized solution in one pass
        var numBytes = (int)Math.ceil(numBooleans / 8d);
        var values = decodeByteRle(buffer, numBytes, pos);
        var bitVector = new BitVector(values, numBooleans);

        var nullableBitset = new BitSet(nullabilityBuffer.size());
        var valueCounter = 0;
        for(var i = 0; i < nullabilityBuffer.size(); i++){
            if(nullabilityBuffer.get(i)){
                var value = bitVector.get(valueCounter++);
                nullableBitset.set(i, value);
            }
            else{
                nullableBitset.set(i, false);
            }
        }

        return ByteBuffer.wrap(nullableBitset.toByteArray());
    }

    public static ByteBuffer decodeByteRle(byte[] buffer, int numBytesResult, IntWrapper pos) {
        ByteBuffer values = ByteBuffer.allocate(numBytesResult);

        var offset = pos.get();
        int valueOffset = 0;
        while (valueOffset < numBytesResult) {
            int header = buffer[offset++] & 0xFF;
            if (header <= 0x7F) {
                /* Runs */
                int numRuns = header + 3;
                byte value = buffer[offset++];
                int endValueOffset = valueOffset + numRuns;
                for (int i = valueOffset; i < endValueOffset; i++) {
                    values.put(i, value);
                }
                valueOffset = endValueOffset;
            } else {
                /* Literals */
                int numLiterals = 256 - header;
                for (int i = 0; i < numLiterals; i++) {
                    byte value = buffer[offset++];
                    values.put(valueOffset++, value);
                }
                //TODO: use System.arrayCopy
                //System.arraycopy(buffer, offset, values.array(), valueOffset, numLiterals);
            }
        }

        pos.set(offset);
        return values;
    }

    /*
    public static ByteBuffer decodeNullableBooleanRle(byte[] buffer, int numBooleans, IntWrapper pos, BitVector bitVector) {
        var numBytes = (int)Math.ceil(numBooleans / 8d);
        return decodeNullableByteRle(buffer, numBytes, pos, bitVector);
    }

    public static ByteBuffer decodeNullableByteRle(byte[] buffer, int numBytesResult, IntWrapper pos, BitVector bitVector) {
        ByteBuffer values = ByteBuffer.allocate(numBytesResult);
        var offset = pos.get();
        int valueOffset = 0;
        while (valueOffset < numBytesResult) {
            int header = buffer[offset++] & 0xFF;
            if (header <= 0x7F) {
                int numRuns = header + 3;
                byte value = buffer[offset++];
                int endValueOffset = valueOffset + numRuns;
                for (int i = valueOffset; i < endValueOffset; i++) {
                    values.put(i, value);

                    if(bitVector.get(i)){
                        values.put(i, value);
                    }
                    else{
                        values.put(i, 0);
                        offset++;
                    }
                }
                valueOffset = endValueOffset;

            } else {
                int numLiterals = 256 - header;
                for (int i = 0; i < numLiterals; i++) {
                    byte value = buffer[offset++];
                    values.put(valueOffset++, value);
                }
                //TODO: use System.arrayCopy
                //System.arraycopy(buffer, offset, values.array(), valueOffset, numLiterals);
            }
        }

        pos.set(offset);
        return values;
    }

    */

    //TODO: implement vectorized solution
    /*public static ByteBuffer decodeByteRleVectorized(byte[] buffer, int numBytesResult, IntWrapper pos) {
        ByteBuffer values = ByteBuffer.allocate(numBytesResult);

        var offset = pos.get();
        int valueOffset = 0;
        var position = 0;
        while (valueOffset < numBytesResult) {
            int header = buffer[offset++] & 0xFF;
            if (header <= 0x7F) {
                int count = header + 3;
                byte value = buffer[offset++];
                int endValueOffset = valueOffset + count;

                ByteVector runVector = ByteVector.fromArray(ByteVector.SPECIES_PREFERRED, buffer, valueOffset);

                int i = 0;
                for (; i <= count; i += ByteVector.SPECIES_PREFERRED.length()) {
                    runVector.intoArray(values, pos + i);
                }

                pos += count;

                valueOffset = endValueOffset;
            } else {
                int numLiterals = 256 - header;
                ByteVector literalVector = ByteVector.fromArray(ByteVector.SPECIES_PREFERRED, buffer, valueOffset);
                int i = 0;
                for (; i <= numLiterals; i += ByteVector.SPECIES_PREFERRED.length()) {
                    literalVector.intoByteBuffer(values, position + i, ByteOrder.LITTLE_ENDIAN);
                }
            }
        }

        values.position(0);
        pos.add(offset);
        return values;
    }*/

    public static IntBuffer decodeFastPfor(byte[] buffer, int numValues, int byteLength, IntWrapper offset){
        /*
        * Create a vectorized conversion from the ByteBuffer to the IntBuffer
        *
        * */

        //TODO: get rid of that conversion
        IntBuffer intBuf =
                ByteBuffer.wrap(buffer, offset.get(), byteLength)
                        .order(ByteOrder.BIG_ENDIAN)
                        .asIntBuffer();
        var bufferSize = (int)Math.ceil(byteLength / 4);
        int[] intValues = new int[bufferSize];
        for(var i = 0; i < intValues.length; i++){
            intValues[i] = intBuf.get(i);
        }

        int[] decodedValues = new int[numValues];
        IntegerCODEC ic = new Composition(new FastPFOR(), new VariableByte());
        ic.uncompress(intValues, new IntWrapper(0), intValues.length, decodedValues, new IntWrapper(0));

        offset.add(byteLength);
        return IntBuffer.wrap(decodedValues);
    }

    /** Varint decoding ----------------------------------------------------------------------------------------*/

    public static IntBuffer decodeVarint(byte[] src, IntWrapper pos, int numValues){
        var values = new int[numValues];
        for(var i = 0; i < numValues; i++){
            var offset = decodeVarint(src, pos.get(), values, i);
            pos.set(offset);
        }
        return IntBuffer.wrap(values);
    }

    private static int decodeVarint(byte[] src, int offset, int[] dst, int dstOffset) {
        /*
         * Max 4 bytes supported.
         * */
        var b= src[offset++];
        var value = b & 0x7f;
        if ((b & 0x80) == 0) {
            dst[dstOffset] = value;
            return offset;
        }

        b = src[offset++];
        value |= (b & 0x7f) << 7;
        if ((b & 0x80) == 0) {
            dst[dstOffset] = value;
            return offset;
        }

        b = src[offset++];
        value |= (b & 0x7f) << 14;
        if ((b & 0x80) == 0) {
            dst[dstOffset] = value;
            return offset;
        }

        b = src[offset++];
        value |= (b & 0x7f) << 21;
        dst[dstOffset] = value;
        return offset;
    }

    /* Source: https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/longcompression/LongVariableByte.java */
    /*public static LongBuffer decodeLongVarint(byte[] in, IntWrapper pos, int numValues) {
        var out = new long[numValues];
        int p = pos.get();
        int finalp = pos.get() + numValues;
        int tmpoutpos = 0;
        for (long v = 0; p < finalp; out[tmpoutpos++] = v) {
            v = in[p] & 0x7F;
            if (in[p] < 0) {
                p += 1;
                continue;
            }
            v = ((in[p + 1] & 0x7F) << 7) | v;
            if (in[p + 1] < 0) {
                p += 2;
                continue;
            }
            v = ((in[p + 2] & 0x7F) << 14) | v;
            if (in[p + 2] < 0 ) {
                p += 3;
                continue;
            }
            v = ((in[p + 3] & 0x7F) << 21) | v;
            if (in[p + 3] < 0) {
                p += 4;
                continue;
            }
            v = (((long) in[p + 4] & 0x7F) << 28) | v;
            if (in[p + 4] < 0) {
                p += 5;
                continue;
            }
            v = (((long) in[p + 5] & 0x7F) << 35) | v;
            if (in[p + 5] < 0) {
                p += 6;
                continue;
            }
            v = (((long) in[p + 6] & 0x7F) << 42) | v;
            if (in[p + 6] < 0) {
                p += 7;
                continue;
            }
            v = (((long) in[p + 7] & 0x7F) << 49) | v;
            if (in[p + 7] < 0) {
                p += 8;
                continue;
            }
            v = (((long) in[p + 8] & 0x7F) << 56) | v;
            if (in[p + 8] < 0) {
                p += 9;
                continue;
            }
            v = (((long) in[p + 9] & 0x7F) << 63) | v;
            p += 10;
        }

        pos.set(p);
        return LongBuffer.wrap(out);
    }*/

    //TODO: refactor for performance
    public static LongBuffer decodeLongVarint(byte[] src, IntWrapper pos, int numValues){
        var values = new long[numValues];
        for(var i = 0; i < numValues; i++){
            long value = 0;
            int shift = 0;
            int index = pos.get();
            while (index < src.length) {
                byte b = src[index++];
                value |= (long) (b & 0x7F) << shift;
                if ((b & 0x80) == 0) {
                    break;
                }
                shift += 7;
                if (shift >= 64) {
                    throw new IllegalArgumentException("Varint too long");
                }
            }

            pos.set(index);
            values[i] = value;
        }

        return LongBuffer.wrap(values);
    }

    /** Rle decoding --------------------------------------------------------------------------------------*/

    public static IntBuffer decodeRle(int[] data, StreamMetadata streamMetadata, boolean isSigned){
        var rleMetadata = (RleEncodedStreamMetadata)streamMetadata;
        return isSigned? VectorizedDecodingUtils.decodeZigZagRLE(data, rleMetadata.runs(), rleMetadata.numRleValues()) :
                VectorizedDecodingUtils.decodeUnsignedRLE(data, rleMetadata.runs(), rleMetadata.numRleValues());
    }

    //TODO: use vectorized solution which is 2x faster in the tests
    public static IntBuffer decodeUnsignedRLE(int[] data, int numRuns, int numTotalValues){
        var values = new int[numTotalValues];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            for(var j = offset; j < offset + runLength; j++){
                values[j] = value;
            }

            offset += runLength;
        }

        return IntBuffer.wrap(values);
    }

    public static IntBuffer decodeZigZagRLE(int[] data, int numRuns, int numTotalValues){
        var values = new int[numTotalValues];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            value = (value >>> 1) ^ ((value << 31) >> 31);
            for(var j = offset; j < offset + runLength; j++){
                values[j] = value;
            }

            offset += runLength;
        }

        return IntBuffer.wrap(values);
    }

    public static LongBuffer decodeRle(long[] data, StreamMetadata streamMetadata, boolean isSigned){
        var rleMetadata = (RleEncodedStreamMetadata)streamMetadata;
        return isSigned? VectorizedDecodingUtils.decodeZigZagRLE(data, rleMetadata.runs(), rleMetadata.numRleValues()) :
                VectorizedDecodingUtils.decodeUnsignedRLE(data, rleMetadata.runs(), rleMetadata.numRleValues());
    }

    public static LongBuffer decodeUnsignedRLE(long[] data, int numRuns, int numTotalValues){
        var values = new long[numTotalValues];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            for(var j = offset; j < offset + runLength; j++){
                values[j] = value;
            }

            offset += runLength;
        }

        return LongBuffer.wrap(values);
    }

    public static LongBuffer decodeZigZagRLE(long[] data, int numRuns, int numTotalValues){
        var values = new long[numTotalValues];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            value = (value >>> 1) ^ ((value << 63) >> 63);
            for(var j = offset; j < offset + runLength; j++){
                values[j] = value;
            }

            offset += runLength;
        }

        return LongBuffer.wrap(values);
    }

    /** Nullable Rle decoding --------------------------------------------------------------------------------------*/

    public static IntBuffer decodeNullableRle(int[] data, StreamMetadata streamMetadata, boolean isSigned,
                                              BitVector bitVector){
        var rleMetadata = (RleEncodedStreamMetadata)streamMetadata;
        return isSigned? VectorizedDecodingUtils.decodeNullableZigZagRLE(bitVector, data, rleMetadata.runs()) :
                VectorizedDecodingUtils.decodeNullableUnsignedRLE(bitVector, data, rleMetadata.runs());
    }

    public static IntBuffer decodeNullableUnsignedRLE(BitVector bitVector, int[] data, int numRuns){
        var values = new int[bitVector.size()];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            for(var j = offset; j < offset + runLength; j++){
                /** There can be null values in a run */
                if(bitVector.get(j)){
                    values[j] = value;
                }
                else{
                    values[j] = 0;
                    offset++;
                }
            }
            offset += runLength;
        }

        return IntBuffer.wrap(values);
    }

    public static IntBuffer decodeNullableZigZagRLE(BitVector bitVector, int[] data, int numRuns){
        var values = new int[bitVector.size()];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            value = (value >>> 1) ^ ((value << 31) >> 31);
            for(var j = offset; j < offset + runLength; j++){
                /** There can be null values in a run */
                if(bitVector.get(j)){
                    values[j] = value;
                }
                else{
                    values[j] = 0;
                    offset++;
                }
            }
            offset += runLength;
        }

        return IntBuffer.wrap(values);
    }

    public static LongBuffer decodeNullableRle(long[] data, StreamMetadata streamMetadata, boolean isSigned,
                                              BitVector bitVector){
        var rleMetadata = (RleEncodedStreamMetadata)streamMetadata;
        return isSigned? VectorizedDecodingUtils.decodeNullableZigZagRLE(bitVector, data, rleMetadata.runs()) :
                VectorizedDecodingUtils.decodeNullableUnsignedRLE(bitVector, data, rleMetadata.runs());
    }

    public static LongBuffer decodeNullableUnsignedRLE(BitVector bitVector, long[] data, int numRuns){
        var values = new long[bitVector.size()];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            for(var j = offset; j < offset + runLength; j++){
                /** There can be null values in a run */
                if(bitVector.get(j)){
                    values[j] = value;
                }
                else{
                    values[j] = 0;
                    offset++;
                }
            }
            offset += runLength;
        }

        return LongBuffer.wrap(values);
    }

    public static LongBuffer decodeNullableZigZagRLE(BitVector bitVector, long[] data, int numRuns){
        var values = new long[bitVector.size()];
        var offset = 0;
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            value = (value >>> 1) ^ ((value << 63) >> 63);
            for(var j = offset; j < offset + runLength; j++){
                /** There can be null values in a run */
                if(bitVector.get(j)){
                    values[j] = value;
                }
                else{
                    values[j] = 0;
                    offset++;
                }
            }
            offset += runLength;
        }

        return LongBuffer.wrap(values);
    }

    public static int decodeUnsignedConstRLE(int[] data){
        return data[1];
    }

    public static int decodeZigZagConstRLE(int[] data){
        var value = data[1];
        return (value >>> 1) ^ ((value << 31) >> 31);
    }

    public static long decodeUnsignedConstRLE(long[] data){
        return data[1];
    }

    public static long decodeZigZagConstRLE(long[] data){
        var value = data[1];
        return (value >>> 1) ^ ((value << 63) >> 63);
    }

    /** Delta encoding  ------------------------------------------------------------------------------*/

    /**
     * In place decoding of the zigzag encoded delta values.
     * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
     */
    public static void decodeZigZagDelta(int[] data) {
        data[0] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31);
        int sz0 = data.length / 4 * 4;
        int i = 1;
        if (sz0 >= 4) {
            for (; i < sz0 - 4; i += 4) {
                var data1 = data[i];
                var data2 = data[i+1];
                var data3 = data[i+2];
                var data4 = data[i+3];

                data[i] = ((data1 >>> 1) ^ ((data1 << 31) >> 31)) + data[i-1];
                data[i+1] = ((data2 >>> 1) ^ ((data2 << 31) >> 31))  + data[i];
                data[i+2] = ((data3 >>> 1) ^ ((data3 << 31) >> 31))  + data[i+1];
                data[i+3] = ((data4 >>> 1) ^ ((data4 << 31) >> 31))  + data[i+2];
            }
        }

        for (; i != data.length; ++i) {
            data[i] = ((data[i] >>> 1) ^ ((data[i] << 31) >> 31)) + data[i - 1];
        }
    }

    /**
     * In place decoding of the zigzag delta encoded Vec2.
     * Inspired by https://github.com/lemire/JavaFastPFOR/blob/master/src/main/java/me/lemire/integercompression/differential/Delta.java
     */
    public static void decodeComponentwiseDeltaVec2(int[] data) {
        data[0] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31) ;
        data[1] = (data[1] >>> 1) ^ ((data[1] << 31) >> 31) ;
        int sz0 = data.length / 4 * 4;
        int i = 2;
        if (sz0 >= 4) {
            for (; i < sz0 - 4; i += 4) {
                var x1 = data[i];
                var y1 = data[i+1];
                var x2 = data[i+2];
                var y2 = data[i+3];

                data[i] = ((x1 >>> 1) ^ ((x1 << 31) >> 31)) + data[i-2];
                data[i+1] = ((y1 >>> 1) ^ ((y1 << 31) >> 31))  + data[i-1];
                data[i+2] = ((x2 >>> 1) ^ ((x2 << 31) >> 31))  + data[i];
                data[i+3] = ((y2 >>> 1) ^ ((y2 << 31) >> 31))  + data[i+1];
            }
        }

        for (; i != data.length; i+=2) {
            data[i] = ((data[i] >>> 1) ^ ((data[i] << 31) >> 31)) + data[i - 2];
            data[i+1] = ((data[i+1] >>> 1) ^ ((data[i+1] << 31) >> 31)) + data[i - 1];
        }
    }

    public static void decodeZigZagDelta(long[] data) {
        data[0] = (data[0] >>> 1) ^ ((data[0] << 63) >> 63);
        int sz0 = data.length / 4 * 4;
        int i = 1;
        if (sz0 >= 4) {
            for (; i < sz0 - 4; i += 4) {
                var data1 = data[i];
                var data2 = data[i+1];
                var data3 = data[i+2];
                var data4 = data[i+3];

                data[i] = ((data1 >>> 1) ^ ((data1 << 63) >> 63)) + data[i-1];
                data[i+1] = ((data2 >>> 1) ^ ((data2 << 63) >> 63))  + data[i];
                data[i+2] = ((data3 >>> 1) ^ ((data3 << 63) >> 63))  + data[i+1];
                data[i+3] = ((data4 >>> 1) ^ ((data4 << 63) >> 63))  + data[i+2];
            }
        }

        for (; i != data.length; ++i) {
            data[i] = ((data[i] >>> 1) ^ ((data[i] << 63) >> 63)) + data[i - 1];
        }
    }

    public static int[] decodeNullableZigZagDelta(BitVector bitVector, int[] data) {
        var decodedData = new int[bitVector.size()];
        var dataCounter = 0;
        if(bitVector.get(0)){
            decodedData[0] = bitVector.get(0) ? ((data[0] >>> 1) ^ ((data[0] << 31) >> 31)) : 0;
            dataCounter = 1;
        }
        else{
            decodedData[0] = 0;
        }

        var i = 1;
        for (; i != decodedData.length; ++i) {
            decodedData[i] = bitVector.get(i)? decodedData[i-1] + ((data[dataCounter] >>> 1) ^ ((data[dataCounter++] << 31) >> 31)) :
                    decodedData[i-1];
        }

        return decodedData;
    }

    public static long[] decodeNullableZigZagDelta(BitVector bitVector, long[] data) {
        var decodedData = new long[bitVector.size()];
        var dataCounter = 0;
        if(bitVector.get(0)){
            decodedData[0] = bitVector.get(0) ? ((data[0] >>> 1) ^ ((data[0] << 63) >> 63)) : 0;
            dataCounter = 1;
        }
        else{
            decodedData[0] = 0;
        }

        var i = 1;
        for (; i != decodedData.length; ++i) {
            decodedData[i] = bitVector.get(i)? decodedData[i-1] + ((data[dataCounter] >>> 1) ^ ((data[dataCounter++] << 63) >> 63)):
                    decodedData[i-1];
        }

        return decodedData;
    }

    /** Decode into offsets by using a BitVector --------------------------------------------------------- */

    /** Delta of Delta encoding to transform a delta encoded length stream into a random accessible offset buffer */
    public static void decodeZigZagDelta(int[] data, BitVector bitVector) {
        data[0] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31);
        int sz0 = data.length / 4 * 4;
        int i = 1;
        if (sz0 >= 4) {
            for (; i < sz0 - 4; i += 4) {

                //10, 12, 8, 18 -> original length
                //10, 2, -4, 10 -> delta encoded
                //10, 12, 8, 18 -> delta decoded -> length
                //10, 22, 30, 48 -> delta of delta decoded -> offset

                //TODO: does length stream really need BitVector
                data[i] =  bitVector.get(i)? ((data[i] >>> 1) ^ ((data[i] << 31) >> 31)) + data[i-1] : 0;

                var data1 = data[i];
                var data2 = data[i+1];
                var data3 = data[i+2];
                var data4 = data[i+3];

                data[i] = ((data1 >>> 1) ^ ((data1 << 31) >> 31)) + data[i-1];
                data[i+1] = ((data2 >>> 1) ^ ((data2 << 31) >> 31))  + data[i];
                data[i+2] = ((data3 >>> 1) ^ ((data3 << 31) >> 31))  + data[i+1];
                data[i+3] = ((data4 >>> 1) ^ ((data4 << 31) >> 31))  + data[i+2];
            }
        }

        for (; i != data.length; ++i) {
            data[i] = ((data[i] >>> 1) ^ ((data[i] << 31) >> 31)) + data[i - 1];
        }
    }

    public static void decodeTopologyStreams(){

    }

    /** Transform data to allow random access --------------------------------------------------------------------- */

    public static int[] zigZagDeltaOfDeltaDecoding(int[] data) {
        var decodedData = new int[data.length + 1];
        decodedData[0] = 0;
        decodedData[1] = (data[0] >>> 1) ^ ((data[0] << 31) >> 31);
        var deltaSum = decodedData[1];
        int i = 2;
        for (; i != decodedData.length; ++i) {
            var zigZagValue = data[i - 1];
            var delta = (zigZagValue >>> 1) ^ ((zigZagValue << 31) >> 31);
            deltaSum += delta;
            decodedData[i] = decodedData[i - 1] + deltaSum;
        }

        return decodedData;
    }

    public static IntBuffer zigZagRleDeltaDecoding(int[] data, int numRuns, int numTotalValues){
        var values = new int[numTotalValues + 1];
        values[0] = 0;
        var offset = 1;
        var previousValue = values[0];
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            value = (value >>> 1) ^ ((value << 31) >> 31);
            for(var j = offset; j < offset + runLength; j++){
                values[j] = value + previousValue;
                previousValue = values[j];
            }

            offset += runLength;
        }

        return IntBuffer.wrap(values);
    }

    public static IntBuffer rleDeltaDecoding(int[] data, int numRuns, int numTotalValues){
        var values = new int[numTotalValues + 1];
        values[0] = 0;
        var offset = 1;
        var previousValue = values[0];
        for(var i = 0; i < numRuns; i++){
            var runLength = data[i];
            var value = data[i + numRuns];
            for(var j = offset; j < offset + runLength; j++){
                values[j] = value + previousValue;
                previousValue = values[j];
            }

            offset += runLength;
        }

        return IntBuffer.wrap(values);
    }

    public static int[] padWithZeros(BitVector bitVector, int[] data) {
        var decodedData = new int[bitVector.size()];
        var dataCounter = 0;
        var i = 0;
        for (; i != decodedData.length; ++i) {
            decodedData[i] = bitVector.get(i)? data[dataCounter++] : 0;
        }

        return decodedData;
    }

    public static int[] padZigZagWithZeros(BitVector bitVector, int[] data) {
        var decodedData = new int[bitVector.size()];
        var dataCounter = 0;
        var i = 0;
        for (; i != decodedData.length; ++i) {
            if(bitVector.get(i)){
                var value = data[dataCounter++];
                decodedData[i] = (value >>> 1) ^ ((value << 31) >> 31);
            }
            else{
                decodedData[i] = 0;
            }
        }

        return decodedData;
    }

    public static long[] padWithZeros(BitVector bitVector, long[] data) {
        var decodedData = new long[bitVector.size()];
        var dataCounter = 0;
        var i = 0;
        for (; i != decodedData.length; ++i) {
            decodedData[i] = bitVector.get(i)? data[dataCounter++] : 0;
        }

        return decodedData;
    }

    public static long[] padZigZagWithZeros(BitVector bitVector, long[] data) {
        var decodedData = new long[bitVector.size()];
        var dataCounter = 0;
        var i = 0;
        for (; i != decodedData.length; ++i) {
            if(bitVector.get(i)){
                var value = data[dataCounter++];
                decodedData[i] = (value >>> 1) ^ ((value << 63) >> 63);
            }
            else{
                decodedData[i] = 0;
            }
        }

        return decodedData;
    }

    public static VectorType getVectorTypeIntStream(StreamMetadata streamMetadata) {
        var logicalLevelTechnique = streamMetadata.logicalLevelTechnique1();
        if(logicalLevelTechnique.equals(LogicalLevelTechnique.RLE)) {
            return ((RleEncodedStreamMetadata)streamMetadata).runs() == 1 ? VectorType.CONST : VectorType.FLAT;
        }

        if(logicalLevelTechnique.equals(LogicalLevelTechnique.DELTA) && streamMetadata.logicalLevelTechnique2().equals(LogicalLevelTechnique.RLE)
            && ((RleEncodedStreamMetadata)streamMetadata).runs() == 1){
            return VectorType.SEQUENCE;
        }

        return streamMetadata.numValues() == 1? VectorType.CONST: VectorType.FLAT;
    }

    public static VectorType getVectorTypeBooleanStream(int numFeatures, int byteLength, byte[] data, IntWrapper offset) {
        var valuesPerRun = 131;
        //TODO: use VectorType metadata field for to test which VectorType is used
        return (Math.ceil((double)numFeatures / valuesPerRun) * 2 == byteLength) &&
                /** Test the first value byte if all bits are set to true */
                (data[offset.get() + 1] & 0xFF) == ((Integer.bitCount(numFeatures) << 2) - 1) ?
                VectorType.CONST : VectorType.FLAT;
    }
}
