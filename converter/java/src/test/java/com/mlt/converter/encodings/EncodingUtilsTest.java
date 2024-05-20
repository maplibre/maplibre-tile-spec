package com.mlt.converter.encodings;

import com.mlt.converter.encodings.EncodingUtils;
import com.mlt.decoder.DecodingUtils;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;

import java.io.IOException;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;

public class EncodingUtilsTest {

    @Test
    public void encodeRle_MixedRunsAndLiterals_ValidEncoding(){
        var values = List.of(1, 1, 1, 2, 4, 5, 8, 8, 8, 8, 9, 9);
        var expectedRuns = List.of(3,1,1,1,4,2);
        var expectedValues = List.of(1,2,4,5,8,9);

        var actualValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());

        assertEquals(expectedRuns, actualValues.getLeft());
        assertEquals(expectedValues, actualValues.getRight());
    }

    @Test
    public void encodeRle_OnlyLiterals_ValidEncoding(){
        var values = List.of(1, 2, 3, 4, 5, 6, 7, 8);
        var expectedRuns = List.of(1, 1, 1, 1, 1, 1, 1, 1);
        var expectedValues = List.of(1, 2, 3, 4, 5, 6, 7, 8);

        var actualValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());

        assertEquals(expectedRuns, actualValues.getLeft());
        assertEquals(expectedValues, actualValues.getRight());
    }

    @Test
    public void encodeRle_OnlyRuns_ValidEncoding(){
        var values = List.of(10, 10, 10, 10, 20, 20, 40, 40, 40, 40);
        var expectedRuns = List.of(4, 2, 4);
        var expectedValues = List.of(10, 20, 40);

        var actualValues = EncodingUtils.encodeRle(values.stream().mapToInt(i -> i).toArray());

        assertEquals(expectedRuns, actualValues.getLeft());
        assertEquals(expectedValues, actualValues.getRight());
    }

    @Test
    public void encodeBooleanRle() throws IOException {
        var numValues = 70;
        var bitset = new BitSet();
        for(var i = 0; i < numValues; i++){
            bitset.set(i, false);
        }

        var encodedBooleans = EncodingUtils.encodeBooleanRle(bitset, numValues);

        var decodeBooleans = DecodingUtils.decodeBooleanRle(encodedBooleans, numValues, encodedBooleans.length, new IntWrapper(0));

        for(var i = 0; i < numValues; i++){
            assertEquals(false, decodeBooleans.get(i));
        }
    }

}
