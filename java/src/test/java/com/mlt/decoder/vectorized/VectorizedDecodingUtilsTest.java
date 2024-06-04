package com.mlt.decoder.vectorized;

import com.mlt.converter.CollectionUtils;
import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.converter.encodings.GeometryEncoder;
import com.mlt.converter.geometry.Vertex;
import com.mlt.vector.BitVector;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.ArrayUtils;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Disabled;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.sql.Array;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.BitSet;
import java.util.concurrent.LinkedTransferQueue;
import java.util.stream.Collectors;
import java.util.stream.IntStream;

import static org.junit.jupiter.api.Assertions.*;

public class VectorizedDecodingUtilsTest {

    @Test
    void decodeBooleanRle_OneByte_DecodesCorrectly() throws IOException {
        BitSet bitSet = new BitSet();
        bitSet.set(0);
        bitSet.set(2);

        byte[] encoded = EncodingUtils.encodeBooleanRle(bitSet, 3);
        var decoded = VectorizedDecodingUtils.decodeBooleanRle(encoded, 3, new IntWrapper(0));

        var bitVector = new BitVector(decoded, 3);
        assertTrue(bitVector.get(0));
        assertFalse(bitVector.get(1));
        assertTrue(bitVector.get(2));
    }

    @Test
    void decodeBooleanRle_FourBytes_DecodesCorrectly() throws IOException {
        BitSet bitSet = new BitSet();
        bitSet.set(1);
        bitSet.set(2);
        bitSet.set(11);
        bitSet.set(26);

        byte[] encoded = EncodingUtils.encodeBooleanRle(bitSet, 27);
        var decoded = VectorizedDecodingUtils.decodeBooleanRle(encoded, 27, new IntWrapper(0));

        var bitVector = new BitVector(decoded, 3);
        assertFalse(bitVector.get(0));
        assertTrue(bitVector.get(1));
        assertTrue(bitVector.get(2));
        assertTrue(bitVector.get(11));
        assertTrue(bitVector.get(26));
        assertFalse(bitVector.get(27));
    }

    @Test
    void decodeLongVarint_UnsignedLong() {
        var values = new long[]{0, 1, 127, 128, 255, 256, 555};

        var encodedValues = EncodingUtils.encodeVarints(values, false, false);
        var pos = new IntWrapper(0);
        var decodedValues = VectorizedDecodingUtils.decodeLongVarint(encodedValues, pos,
                values.length);

        assertEquals(values.length, decodedValues.capacity());
        assertEquals(pos.get(), encodedValues.length);
        for (var i = 0; i < values.length; i++) {
            assertEquals(values[i], decodedValues.get(i));
        }
    }

    @Test
    void decodeZigZagDelta_Int(){
        var values = new int[]{0, 1, -1, 127, -127, 128, -128, 555, -555, 2000, -4000};

        var deltaValues = EncodingUtils.encodeDeltas(values);
        var zigZagValues =  EncodingUtils.encodeZigZag(deltaValues);
        VectorizedDecodingUtils.decodeZigZagDelta(zigZagValues);

        assertEquals(values.length, zigZagValues.length);
        for (var i = 0; i < values.length; i++) {
            assertEquals(values[i], zigZagValues[i]);
        }
    }

    @Test
    void decodeZigZagDelta_Long(){
        var values = new long[]{0, 1, -10, 127, -127, 128, -128, 555, -555};

        var deltaValues = EncodingUtils.encodeDeltas(values);
        var zigZagValues =  EncodingUtils.encodeZigZag(deltaValues);
        VectorizedDecodingUtils.decodeZigZagDelta(zigZagValues);

        assertEquals(values.length, zigZagValues.length);
        for (var i = 0; i < values.length; i++) {
            assertEquals(values[i], zigZagValues[i]);
        }
    }

    @Test
    void decodeNullableZigZagDelta_Int(){
        var values = new int[]{10, 14, 19, 16};
        var expectedValues = new int[]{10, 14, 14, 19, 19, 16, 16};
        var deltas = EncodingUtils.encodeDeltas(values);
        var zigZagDeltas = EncodingUtils.encodeZigZag(deltas);
        //0101011
        var byteBuffer = ByteBuffer.wrap(new byte[]{0x2B});
        var bitVector = new BitVector(byteBuffer, 7);

        var decodedValues = VectorizedDecodingUtils.decodeNullableZigZagDelta(bitVector, zigZagDeltas);

        assertEquals(expectedValues.length, decodedValues.length);
        assertTrue(Arrays.equals(expectedValues, decodedValues));
    }

    @Test @Disabled
    void decodeNullableZigZagDelta_Long(){
        var values = new long[]{-4, -14, 19, -16};
        var expectedValues = new long[]{-4, -14, -14, 19, 19, -16, -16};
        var deltas = EncodingUtils.encodeDeltas(values);
        var zigZagDeltas = EncodingUtils.encodeZigZag(deltas);
        //0101011
        var byteBuffer = ByteBuffer.wrap(new byte[]{0x2B});
        var bitVector = new BitVector(byteBuffer, 7);

        var decodedValues = VectorizedDecodingUtils.decodeNullableZigZagDelta(bitVector, zigZagDeltas);

        assertEquals(expectedValues.length, decodedValues.length);
        assertTrue(Arrays.equals(expectedValues, decodedValues));
    }

    @Test
    void decodeNullableUnsignedRLE_Int(){
        var values = new int[]{1, 1, 2, 2};
        var expectedValues = new int[]{1, 1, 0, 2, 0, 2, 0};
        //0101011
        var byteBuffer = ByteBuffer.wrap(new byte[]{0x2B});
        var bitVector = new BitVector(byteBuffer, 7);
        var rleComponentBuffer = EncodingUtils.encodeRle(values);
        var rleList = new ArrayList<>(rleComponentBuffer.getLeft());
        rleList.addAll(rleComponentBuffer.getRight());
        var rleBuffer = rleList.stream().mapToInt(i -> i).toArray();

        var decodedValues = VectorizedDecodingUtils.decodeNullableUnsignedRLE(bitVector, rleBuffer,
                rleComponentBuffer.getLeft().size());

        assertEquals(expectedValues.length, decodedValues.capacity());
        assertTrue(Arrays.equals(expectedValues, decodedValues.array()));
    }

    @Test
    void decodeNullableUnsignedRLE_Long(){
        var values = new long[]{1, 4, 2, 2};
        var expectedValues = new long[]{1, 4, 0, 2, 0, 2, 0};
        //0101011
        var byteBuffer = ByteBuffer.wrap(new byte[]{0x2B});
        var bitVector = new BitVector(byteBuffer, 7);
        var rleComponentBuffer = EncodingUtils.encodeRle(values);
        var rleList = new ArrayList<>(rleComponentBuffer.getLeft().stream().mapToLong(i -> i).boxed().collect(Collectors.toList()));
        rleList.addAll(rleComponentBuffer.getRight());
        var rleBuffer = rleList.stream().mapToLong(i -> i).toArray();

        var decodedValues = VectorizedDecodingUtils.decodeNullableUnsignedRLE(bitVector, rleBuffer,
                rleComponentBuffer.getLeft().size());

        assertEquals(expectedValues.length, decodedValues.capacity());
        assertTrue(Arrays.equals(expectedValues, decodedValues.array()));
    }

    @Test
    void decodeFastPfor(){
        var values = new int[]{2, 12};
        var encodedValues = EncodingUtils.encodeFastPfor128(values, false, false);

        var decodedValues = VectorizedDecodingUtils.decodeFastPfor(encodedValues, values.length, encodedValues.length, new IntWrapper(0));

        assertTrue(Arrays.equals(values, decodedValues.array()));
    }

    @Test
    void decodeComponentwiseDeltaVec2(){
        var values = new int[]{10, 15, 20, 25, 30, 35, 20, 25};
        var componentwiseDeltaValues = new int[]{10, 15, 10, 10, 10, 10, -10, -10};
        var zigZagValues = EncodingUtils.encodeZigZag(componentwiseDeltaValues);

        VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(zigZagValues);

        assertTrue(Arrays.equals(values, zigZagValues));
    }

    @Test
    void decodeComponentwiseDeltaVec2_realData(){
        var values = new int[]{3277, 6952, 6554, -145, 3277, 6952, 6554, -145};
        var vertices = IntStream.range(0, values.length / 2).mapToObj(i -> new Vertex(values[i*2], values[i*2+1])).
                collect(Collectors.toList());
        var zigZagValues = GeometryEncoder.zigZagDeltaEncodeVertices(vertices);

        VectorizedDecodingUtils.decodeComponentwiseDeltaVec2(zigZagValues);

        assertTrue(Arrays.equals(values, zigZagValues));
    }

}
