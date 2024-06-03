package com.mlt.decoder;

import com.mlt.converter.encodings.*;
import com.mlt.metadata.stream.*;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.junit.jupiter.api.Disabled;

import java.util.List;

public class IntegerDecoderTest {

    @Test
    public void decodeIntStream_SignedIntegerValues_PlainFastPforEncode(){
        var values = List.of(1, 2, 7 ,3 , -4 , 5, 1, -8);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.FAST_PFOR, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.NONE, streamMetadata.logicalLevelTechnique1());
    }

    @Test
    public void decodeIntStream_SignedIntegerValues_PlainVarintEncode(){
        var values = List.of(1, 2, 7 ,3 , -4 , 5, 1, -8);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.VARINT, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.NONE, streamMetadata.logicalLevelTechnique1());
    }

    @Test @Disabled
    public void decodeIntStream_SignedIntegerValues_FastPforDeltaRleEncode(){
        var values = List.of(-1, -2, -3 , -4 , -5 , -6, -7, 8);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.FAST_PFOR, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
    }

    @Test @Disabled
    public void decodeIntStream_SignedIntegerValues_VarintDeltaRleEncode(){
        var values = List.of(-1, -2, -3 , -4 , -5 , -6, -7, 8);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.VARINT, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
    }

    @Test @Disabled
    public void decodeIntStream_SignedIntegerValues_FastPforRleEncode(){
        var values = List.of(-1, -1, -1 , -1 , -1 , -1, -2, -2);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.FAST_PFOR, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
    }

    @Test @Disabled
    public void decodeIntStream_SignedIntegerValues_VarintRleEncode(){
        var values = List.of(-1, -1, -1 , -1 , -1 , -1, -2, -2);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.VARINT, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
    }

    @Test @Disabled
    public void decodeIntStream_UnsignedIntegerValues_VarintRleEncode(){
        var values = List.of(1, 1, 1 , 1 , 1 , 1, 2, 2);
        var encodedStream = IntegerEncoder.encodeIntStream(values, PhysicalLevelTechnique.VARINT, true,
                PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeIntStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
    }

    @Test @Disabled
    public void decodeLongStream_SignedIntegerValues_PlainEncode(){
        var values = List.of(1l, 2l, 7l ,3l , -4l , 5l, 1l, -8l);
        var encodedStream = IntegerEncoder.encodeLongStream(values, true, PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeLongStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.NONE, streamMetadata.logicalLevelTechnique1());
    }

    @Test @Disabled
    public void decodeLongStream_SignedIntegerValues_DeltaRleEncode(){
        var values = List.of(-1l, -2l, -3l , -4l , -5l , -6l, -7l, 8l);
        var encodedStream = IntegerEncoder.encodeLongStream(values, true, PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeLongStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
    }

    @Test @Disabled
    public void decodeLongStream_SignedIntegerValues_RleEncode(){
        var values = List.of(-1l, -1l, -1l , -1l , -1l , -1l, -2l, -2l);
        var encodedStream = IntegerEncoder.encodeLongStream(values, true, PhysicalStreamType.DATA, null);

        var offset = new IntWrapper(0);
        var streamMetadata = StreamMetadata.decode(encodedStream, offset);
        var decodedValues = IntegerDecoder.decodeLongStream(encodedStream, offset, streamMetadata, true);

        Assert.equals(values, decodedValues);
        Assert.equals(LogicalLevelTechnique.RLE, streamMetadata.logicalLevelTechnique1());
    }

}
