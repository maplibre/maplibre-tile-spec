package com.mlt.decoder.vectorized;

import com.mlt.metadata.stream.DictionaryType;
import com.mlt.metadata.stream.MortonEncodedStreamMetadata;
import com.mlt.metadata.stream.StreamMetadata;
import com.mlt.metadata.stream.StreamMetadataDecoder;
import com.mlt.vector.VectorType;
import com.mlt.vector.geometry.GeometryVector;
import com.mlt.vector.geometry.TopologyVector;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.NotImplementedException;

import java.nio.IntBuffer;
import java.util.Optional;

public class VectorizedGeometryDecoder {
    public record MortonSettings(int numBits, int coordinateShift){}

    public record GeometryColumn(IntBuffer geometryTypes, IntBuffer numGeometries, IntBuffer numParts, IntBuffer numRings,
                          IntBuffer vertexOffsets, IntBuffer vertexBuffer, Optional<MortonSettings> mortonInfo){}

    private VectorizedGeometryDecoder(){}

    public static VectorizedGeometryDecoder.GeometryColumn decodeGeometryColumn(byte[] tile, int numStreams, IntWrapper offset) {
        var geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
        //TODO: use byte rle encoding
        var geometryTypes = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);

        IntBuffer numGeometries = null;
        IntBuffer numParts = null;
        IntBuffer numRings = null;
        IntBuffer vertexOffsets = null;
        IntBuffer vertexBuffer = null;
        Optional<MortonSettings> mortonSettings = Optional.empty();
        for(var i = 0; i < numStreams - 1; i++){
            var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            switch (geometryStreamMetadata.physicalStreamType()){
                case LENGTH:
                    switch (geometryStreamMetadata.logicalStreamType().lengthType()){
                        case GEOMETRIES:
                            numGeometries = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case PARTS:
                            numParts = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case RINGS:
                            numRings = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                            break;
                        case TRIANGLES:
                            throw new NotImplementedException("Not implemented yet.");
                    }
                    break;
                case OFFSET:
                    vertexOffsets = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                    break;
                case DATA:
                    if(DictionaryType.VERTEX.equals(geometryStreamMetadata.logicalStreamType().dictionaryType())){
                        vertexBuffer = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, true);
                    }
                    else{
                        var mortonMetadata = (MortonEncodedStreamMetadata)geometryStreamMetadata;
                        mortonSettings = Optional.of(new MortonSettings(mortonMetadata.numBits(), mortonMetadata.coordinateShift()));
                        vertexBuffer = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                    }
                    break;
            }
        }

        return new VectorizedGeometryDecoder.GeometryColumn(geometryTypes, numGeometries, numParts, numRings, vertexOffsets,
                vertexBuffer, mortonSettings);
    }

    //TODO: get rid fo numFeatures parameter
    public static GeometryVector decodeToRandomAccessFormat(byte[] tile, int numStreams, IntWrapper offset, int numFeatures) {
        var geometryTypeMetadata = StreamMetadataDecoder.decode(tile, offset);
        var geometryTypesVectorType = VectorizedDecodingUtils.getVectorTypeIntStream(geometryTypeMetadata);

        IntBuffer numGeometries = null;
        IntBuffer numParts = null;
        IntBuffer numRings = null;
        IntBuffer vertexOffsets = null;
        IntBuffer vertexBuffer = null;
        Optional<MortonSettings> mortonSettings = Optional.empty();

        if(geometryTypesVectorType.equals(VectorType.CONST)){
            /** All geometries in the colum have the same geometry type */
            var geometryType = VectorizedIntegerDecoder.decodeConstIntStream(tile, offset, geometryTypeMetadata, false);

            for(var i = 0; i < numStreams - 1; i++){
                var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
                switch (geometryStreamMetadata.physicalStreamType()){
                    case LENGTH:
                        switch (geometryStreamMetadata.logicalStreamType().lengthType()){
                            case GEOMETRIES:
                                numGeometries = VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(tile, offset,
                                        geometryStreamMetadata);
                                break;
                            case PARTS:
                                numParts = VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(tile, offset,
                                        geometryStreamMetadata);
                                break;
                            case RINGS:
                                numRings = VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(tile, offset,
                                        geometryStreamMetadata);
                                break;
                            case TRIANGLES:
                                throw new NotImplementedException("Not implemented yet.");
                        }
                        break;
                    case OFFSET:
                        vertexOffsets = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                    case DATA:
                        if(DictionaryType.VERTEX.equals(geometryStreamMetadata.logicalStreamType().dictionaryType())){
                            vertexBuffer = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, true);
                        }
                        else{
                            var mortonMetadata = (MortonEncodedStreamMetadata)geometryStreamMetadata;
                            mortonSettings = Optional.of(new MortonSettings(mortonMetadata.numBits(), mortonMetadata.coordinateShift()));
                            vertexBuffer = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        }
                        break;
                }
            }

            return mortonSettings.isPresent()? GeometryVector.createConstMortonEncodedGeometryVector(
                    numFeatures, geometryType,
                    new TopologyVector(numGeometries, numParts, numRings), vertexOffsets, vertexBuffer) :
                    /** Currently only 2D coordinates (Vec2) are implemented in the encoder  */
                    GeometryVector.createConst2DGeometryVector(numFeatures, geometryType,
                            new TopologyVector(numGeometries, numParts, numRings), vertexOffsets, vertexBuffer);
        }

        /** Different geometry types are mixed in the geometry column */
        var geometryTypeVector = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryTypeMetadata, false);

        for(var i = 0; i < numStreams - 1; i++){
            var geometryStreamMetadata = StreamMetadataDecoder.decode(tile, offset);
            switch (geometryStreamMetadata.physicalStreamType()){
                    case LENGTH:
                        switch (geometryStreamMetadata.logicalStreamType().lengthType()){
                            case GEOMETRIES:
                                numGeometries = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                                break;
                            case PARTS:
                                numParts = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                                break;
                            case RINGS:
                                numRings = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                                break;
                            case TRIANGLES:
                                throw new NotImplementedException("Not implemented yet.");
                        }
                        break;
                    case OFFSET:
                        vertexOffsets = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        break;
                    case DATA:
                        if(DictionaryType.VERTEX.equals(geometryStreamMetadata.logicalStreamType().dictionaryType())){
                            vertexBuffer = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, true);
                        }
                        else{
                            var mortonMetadata = (MortonEncodedStreamMetadata)geometryStreamMetadata;
                            mortonSettings = Optional.of(new MortonSettings(mortonMetadata.numBits(), mortonMetadata.coordinateShift()));
                            vertexBuffer = VectorizedIntegerDecoder.decodeIntStream(tile, offset, geometryStreamMetadata, false);
                        }
                        break;
                }
        }

            //TODO: refactor the following instructions -> decode in one pass for performance reasons
        if(numGeometries != null){
            numGeometries = decodeGeometryLengthStream(geometryTypeVector, numGeometries);
        }

        if(numParts != null){
            var partsBufferSize = numGeometries != null? numGeometries.get(numGeometries.capacity() - 1) :
                    numParts.capacity();
            numParts = decodeTopologyLengthStream(geometryTypeVector, numParts, partsBufferSize,
                    numRings != null? 1 : 0, numGeometries);
        }

        if(numRings != null){
            /** Only ringOffset stream not possible -> always in combination with geometryOffsets and partOffsets */
            var ringBufferSize = numParts.get(numParts.capacity() - 1);
            numRings = decodeTopologyLengthStream(geometryTypeVector, numParts, ringBufferSize, 0, numGeometries);
        }

        return mortonSettings.isPresent()? GeometryVector.createMortonEncodedGeometryVector(geometryTypeVector,
                new TopologyVector(numGeometries, numParts, numRings), vertexOffsets, vertexBuffer) :
                /** Currently only 2D coordinates (Vec2) are implemented in the encoder  */
                GeometryVector.create2DGeometryVector(geometryTypeVector, new TopologyVector(numGeometries, numParts, numRings),
                        vertexOffsets, vertexBuffer);
    }

    private static IntBuffer decodeGeometryLengthStream(IntBuffer geometryTypes, IntBuffer numGeometries){
        var geometryOffsets = new int[geometryTypes.capacity() + 1];
        var previousOffset = 0;
        geometryOffsets[0] = previousOffset;
        var geometryCounter = 0;
        for(var i = 0; i < geometryTypes.capacity(); i++){
           geometryOffsets[i] =  previousOffset + (geometryTypes.get(i) > 2? numGeometries.get(geometryCounter++): 1);
           previousOffset = geometryOffsets[i];
        }

        return IntBuffer.wrap(geometryOffsets);
    }

    /**
     * @param streamId 1 for numParts buffer and 0 for numRings buffer
     */
    private static IntBuffer decodeTopologyLengthStream(IntBuffer geometryTypes, IntBuffer topologyLengthBuffer, int topologyOffsetsBufferSize,
                                                        int streamId, IntBuffer geometryOffsetBuffer){
        /** TODO: refactor -> create a more efficient solution as this quick and dirty implementation */
        var topologyOffsetsBuffer = new int[topologyOffsetsBufferSize + 1];
        topologyOffsetsBuffer[0] = 0;
        for(var i = 1; i < geometryTypes.capacity()+1; i++){
            var geometryType = geometryTypes.get(i);
            var previousOffset = geometryOffsetBuffer.get(i-1);
            if(geometryType <= 2){
                /** Handle single part geometry types -> Point, LineString, Polygon
                 *  case1: value exists in specific topology buffer (PartOffsets or RingsOffsets)
                 *  -> always the case for Polygons and can be the case for LineStrings
                 *  case2: There is no value in the current topology stream
                 *  -> for example for Point geometry or LineString when current stream is PartOffsets
                 *  and an additional RingOffsets stream is present
                 * */
                topologyOffsetsBuffer[i] = previousOffset +
                        ((geometryType > streamId)? topologyLengthBuffer.get(i): 1);
            }
            else{
                /** Handle multipart geometry -> MultiPoint, MultiLineString, MultiPolygon */
                if(geometryType - 3 > streamId){
                    //TODO: merge with first if branch
                    /** value exists in specific topology stream (PartOffsets or RingsOffsets)
                     *  -> always the case for Polygons and can be the case for LineStrings */
                    topologyOffsetsBuffer[i] = previousOffset + topologyLengthBuffer.get(i);
                }
                else{
                    /** There is no value in the current topology stream but a parent stream
                     * has a value for this geometry e.g. partOffsets and ringOffsets streams
                     * for mixed geometryTypes of MultiPolygon and MultiPoint.
                     * Take the value from geometryOffsetsBuffer and repeat as many times as the value.
                     * For example a MultiPolygon and MultiPoint geometry are mixed in column.
                     * When the MultiPoint geometry consists of 5 points then add 5 times one to the PartOffset and
                     * RingOffset stream to enable random access.
                     * */
                    var numGeometries = geometryOffsetBuffer.get(i) - geometryOffsetBuffer.get(i-1);
                    for(var j = 1; j <= numGeometries; j++){
                        topologyOffsetsBuffer[i] = previousOffset + j;
                    }
                }
            }
        }

        return IntBuffer.wrap(topologyOffsetsBuffer);
    }

}
