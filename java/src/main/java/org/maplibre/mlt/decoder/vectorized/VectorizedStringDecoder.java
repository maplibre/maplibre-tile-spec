package org.maplibre.mlt.decoder.vectorized;

import org.maplibre.mlt.converter.Settings;
import org.maplibre.mlt.metadata.stream.DictionaryType;
import org.maplibre.mlt.metadata.stream.LengthType;
import org.maplibre.mlt.metadata.stream.RleEncodedStreamMetadata;
import org.maplibre.mlt.metadata.stream.StreamMetadataDecoder;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import org.maplibre.mlt.vector.BitVector;
import org.maplibre.mlt.vector.Vector;
import org.maplibre.mlt.vector.dictionary.DictionaryDataVector;
import org.maplibre.mlt.vector.dictionary.StringDictionaryVector;
import org.maplibre.mlt.vector.dictionary.StringSharedDictionaryVector;
import org.maplibre.mlt.vector.flat.StringFlatVector;
import org.maplibre.mlt.vector.fsstdictionary.StringFsstDictionaryVector;
import org.maplibre.mlt.vector.fsstdictionary.StringSharedFsstDictionaryVector;
import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import me.lemire.integercompression.IntWrapper;

public class VectorizedStringDecoder {

  private VectorizedStringDecoder() {}

  // TODO: create baseclass
  /** Not optimized for random access only for sequential iteration */
  public static Vector decode(
      String name, byte[] data, IntWrapper offset, int numStreams, BitVector bitVector)
      throws IOException {
    /*
     * String column layouts:
     * -> plain -> present, length, data
     * -> dictionary -> present, length, dictionary, data
     * -> fsst dictionary -> symbolTable, symbolLength, dictionary, length, present, data
     * */

    IntBuffer dictionaryLengthStream = null;
    IntBuffer offsetStream = null;
    ByteBuffer dictionaryStream = null;
    IntBuffer symbolLengthStream = null;
    ByteBuffer symbolTableStream = null;
    for (var i = 0; i < numStreams; i++) {
      var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      switch (streamMetadata.physicalStreamType()) {
        case OFFSET:
          {
            offsetStream =
                VectorizedIntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            break;
          }
        case LENGTH:
          {
            var ls = VectorizedIntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            if (LengthType.DICTIONARY.equals(streamMetadata.logicalStreamType().lengthType())) {
              dictionaryLengthStream = ls;
            } else {
              symbolLengthStream = ls;
            }

            break;
          }
        case DATA:
          {
            var ds = ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength());
            offset.add(streamMetadata.byteLength());
            if (DictionaryType.SINGLE.equals(streamMetadata.logicalStreamType().dictionaryType())) {
              dictionaryStream = ds;
            } else {
              symbolTableStream = ds;
            }
            break;
          }
      }
    }

    if (symbolTableStream != null) {
      return decodeFsstDictionary(
          name,
          bitVector,
          offsetStream,
          dictionaryLengthStream,
          dictionaryStream,
          symbolLengthStream,
          symbolTableStream);
    } else if (dictionaryStream != null) {
      return decodeDictionary(
          name, bitVector, offsetStream, dictionaryLengthStream, dictionaryStream);
    }

    return decodePlain(name, bitVector, offsetStream, dictionaryStream);
  }

  public static Vector decodeToRandomAccessFormat(
      String name,
      byte[] data,
      IntWrapper offset,
      int numStreams,
      BitVector bitVector,
      int numFeatures) {
    // TODO: handle ConstVector
    IntBuffer dictionaryLengthStream = null;
    IntBuffer offsetStream = null;
    ByteBuffer dictionaryStream = null;
    IntBuffer symbolLengthStream = null;

    ByteBuffer symbolTableStream = null;
    for (var i = 0; i < numStreams; i++) {
      var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      if (streamMetadata.byteLength() == 0) {
        continue;
      }

      switch (streamMetadata.physicalStreamType()) {
        case OFFSET:
          {
            boolean isNullable = bitVector != null;
            offsetStream =
                isNullable
                    ? VectorizedIntegerDecoder.decodeNullableIntStream(
                        data, offset, streamMetadata, false, bitVector)
                    : VectorizedIntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            break;
          }
        case LENGTH:
          {
            var ls =
                VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(
                    data, offset, streamMetadata);
            if (LengthType.DICTIONARY.equals(streamMetadata.logicalStreamType().lengthType())) {
              dictionaryLengthStream = ls;
            } else {
              symbolLengthStream = ls;
            }

            break;
          }
        case DATA:
          {
            var ds = ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength());
            offset.add(streamMetadata.byteLength());
            if (DictionaryType.SINGLE.equals(streamMetadata.logicalStreamType().dictionaryType())) {
              dictionaryStream = ds;
            } else {
              symbolTableStream = ds;
            }
            break;
          }
      }
    }

    if (symbolTableStream != null) {
      return StringFsstDictionaryVector.createFromOffsetBuffer(
          name,
          bitVector,
          offsetStream,
          dictionaryLengthStream,
          dictionaryStream,
          symbolLengthStream,
          symbolTableStream);
    } else if (dictionaryStream != null) {
      return bitVector != null
          ? StringDictionaryVector.createNullableVector(
              name, bitVector, offsetStream, dictionaryLengthStream, dictionaryStream)
          : StringDictionaryVector.createNonNullableVector(
              name, offsetStream, dictionaryLengthStream, dictionaryStream, numFeatures);
    }

    return bitVector != null
        ? StringFlatVector.createNonNullableVector(name, bitVector, offsetStream, dictionaryStream)
        : StringFlatVector.createNonNullableVector(
            name, offsetStream, dictionaryStream, numFeatures);
  }

  // TODO: create baseclass for shared dictionary
  /** Not optimized for random access only for sequential iteration */
  public static Vector decodeSharedDictionary(
      byte[] data, IntWrapper offset, MltTilesetMetadata.Column column) {
    IntBuffer dictionaryLengthBuffer = null;
    ByteBuffer dictionaryBuffer = null;
    IntBuffer symbolLengthBuffer = null;
    ByteBuffer symbolTableBuffer = null;

    // TODO: refactor to be spec compliant -> start by decoding the FieldMetadata, StreamMetadata
    // and PresentStream
    boolean dictionaryStreamDecoded = false;
    while (!dictionaryStreamDecoded) {
      var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      switch (streamMetadata.physicalStreamType()) {
        case LENGTH:
          {
            if (LengthType.DICTIONARY.equals(streamMetadata.logicalStreamType().lengthType())) {
              dictionaryLengthBuffer =
                  VectorizedIntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            } else {
              symbolLengthBuffer =
                  VectorizedIntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            }
            break;
          }
        case DATA:
          {
            // TODO: fix -> only shared is allowed in that case
            if (DictionaryType.SINGLE.equals(streamMetadata.logicalStreamType().dictionaryType())
                || DictionaryType.SHARED.equals(
                    streamMetadata.logicalStreamType().dictionaryType())) {
              dictionaryBuffer = ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength());
              dictionaryStreamDecoded = true;
            } else {
              symbolTableBuffer = ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength());
            }

            offset.add(streamMetadata.byteLength());
            break;
          }
      }
    }

    var chieldFields = column.getComplexType().getChildrenList();
    var fieldVectors = new DictionaryDataVector[chieldFields.size()];
    var i = 0;
    for (var childField : chieldFields) {
      var numStreams = VectorizedDecodingUtils.decodeVarint(data, offset, 1).get(0);
      if (numStreams == 0) {
        /* Column is not present in the tile */
        continue;
      }

      if (numStreams != 2
          || childField.hasComplexField()
          || childField.getScalarField().getPhysicalType()
              != MltTilesetMetadata.ScalarType.STRING) {
        throw new IllegalArgumentException(
            "Currently only optional string fields are implemented for a struct.");
      }

      var presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      var presentStream =
          VectorizedDecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(), offset);
      var offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      var offsetStream =
          VectorizedIntegerDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

      var columnName =
          column.getName()
              + (childField.getName().equals("default")
                  ? ""
                  : (Settings.MLT_CHILD_FIELD_SEPARATOR + childField.getName()));
      // TODO: refactor to work also when present stream is null
      var dataVector =
          new DictionaryDataVector(
              columnName,
              new BitVector(presentStream, presentStreamMetadata.numValues()),
              offsetStream);
      fieldVectors[i++] = dataVector;
    }

    return symbolTableBuffer != null
        ? new StringSharedFsstDictionaryVector(
            column.getName(),
            dictionaryLengthBuffer,
            dictionaryBuffer,
            symbolLengthBuffer,
            symbolTableBuffer,
            fieldVectors)
        : new StringSharedDictionaryVector(
            column.getName(), dictionaryLengthBuffer, dictionaryBuffer, fieldVectors);
  }

  public static Vector decodeSharedDictionaryToRandomAccessFormat(
      byte[] data, IntWrapper offset, MltTilesetMetadata.Column column, int numFeatures) {
    IntBuffer dictionaryOffsetBuffer = null;
    ByteBuffer dictionaryBuffer = null;
    IntBuffer symbolOffsetBuffer = null;
    ByteBuffer symbolTableBuffer = null;

    // TODO: refactor to be spec compliant -> start by decoding the FieldMetadata, StreamMetadata
    // and PresentStream
    boolean dictionaryStreamDecoded = false;
    while (!dictionaryStreamDecoded) {
      var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      if (streamMetadata.byteLength() == 0) {
        // TODO: quick and dirty approach -> find proper solution
        // continue;
        System.out.println("error");
      }

      switch (streamMetadata.physicalStreamType()) {
        case LENGTH:
          {
            if (LengthType.DICTIONARY.equals(streamMetadata.logicalStreamType().lengthType())) {
              try {
                dictionaryOffsetBuffer =
                    VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(
                        data, offset, streamMetadata);
              } catch (Exception e) {
                e.printStackTrace();
              }

            } else {
              symbolOffsetBuffer =
                  VectorizedIntegerDecoder.decodeLengthStreamToOffsetBuffer(
                      data, offset, streamMetadata);
            }
            break;
          }
        case DATA:
          {
            // TODO: fix -> only shared is allowed in that case
            if (DictionaryType.SINGLE.equals(streamMetadata.logicalStreamType().dictionaryType())
                || DictionaryType.SHARED.equals(
                    streamMetadata.logicalStreamType().dictionaryType())) {
              dictionaryBuffer = ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength());
              dictionaryStreamDecoded = true;
            } else {
              symbolTableBuffer = ByteBuffer.wrap(data, offset.get(), streamMetadata.byteLength());
            }

            offset.add(streamMetadata.byteLength());
            break;
          }
      }
    }

    var childFields = column.getComplexType().getChildrenList();
    var fieldVectors = new DictionaryDataVector[childFields.size()];
    var i = 0;
    for (var childField : childFields) {
      var numStreams = VectorizedDecodingUtils.decodeVarint(data, offset, 1).get(0);
      if (numStreams == 0) {
        /* Column is not present in the tile */
        continue;
      }

      if (numStreams != 2
          || childField.hasComplexField()
          || childField.getScalarField().getPhysicalType()
              != MltTilesetMetadata.ScalarType.STRING) {
        throw new IllegalArgumentException(
            "Currently only optional string fields are implemented for a struct.");
      }

      var presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      // TODO: check if ConstVector
      var presentStream =
          VectorizedDecodingUtils.decodeBooleanRle(data, presentStreamMetadata.numValues(), offset);
      var offsetStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      // TODO: get rid of that check for rle encoding by using numValues as the number of values
      // independent of the encoding
      boolean isNullable =
          (offsetStreamMetadata instanceof RleEncodedStreamMetadata
                  ? ((RleEncodedStreamMetadata) offsetStreamMetadata).numRleValues()
                  : offsetStreamMetadata.numValues())
              != numFeatures;
      var offsetStream =
          isNullable
              ? VectorizedIntegerDecoder.decodeNullableIntStream(
                  data,
                  offset,
                  offsetStreamMetadata,
                  false,
                  new BitVector(presentStream, presentStreamMetadata.numValues()))
              : VectorizedIntegerDecoder.decodeIntStream(data, offset, offsetStreamMetadata, false);

      var columnName =
          column.getName()
              + (childField.getName().equals("default")
                  ? ""
                  : (Settings.MLT_CHILD_FIELD_SEPARATOR + childField.getName()));
      // TODO: refactor to work also when present stream is null
      var dataVector =
          new DictionaryDataVector(
              columnName,
              new BitVector(presentStream, presentStreamMetadata.numValues()),
              offsetStream);
      fieldVectors[i++] = dataVector;
    }

    return symbolTableBuffer != null
        ? StringSharedFsstDictionaryVector.createFromOffsetBuffer(
            column.getName(),
            dictionaryOffsetBuffer,
            dictionaryBuffer,
            symbolOffsetBuffer,
            symbolTableBuffer,
            fieldVectors)
        : StringSharedDictionaryVector.createFromOffsetBuffer(
            column.getName(), dictionaryOffsetBuffer, dictionaryBuffer, fieldVectors);
  }

  private static StringFlatVector decodePlain(
      String name, BitVector nullabilityVector, IntBuffer lengthStream, ByteBuffer utf8Values) {
    return new StringFlatVector(name, nullabilityVector, lengthStream, utf8Values);
  }

  private static StringDictionaryVector decodeDictionary(
      String name,
      BitVector nullabilityVector,
      IntBuffer dictionaryOffsets,
      IntBuffer lengthStream,
      ByteBuffer utf8Values) {
    return new StringDictionaryVector(
        name, nullabilityVector, dictionaryOffsets, lengthStream, utf8Values);
  }

  private static StringFsstDictionaryVector decodeFsstDictionary(
      String name,
      BitVector nullabilityVector,
      IntBuffer dictionaryOffsets,
      IntBuffer lengthStream,
      ByteBuffer utf8Values,
      IntBuffer symbolLengthStream,
      ByteBuffer symbolTable) {
    return new StringFsstDictionaryVector(
        name,
        nullabilityVector,
        dictionaryOffsets,
        lengthStream,
        utf8Values,
        symbolLengthStream,
        symbolTable);
  }
}
