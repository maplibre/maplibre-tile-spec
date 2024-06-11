package com.mlt.decoder;

import com.mlt.converter.encodings.fsst.FsstEncoder;
import com.mlt.metadata.stream.DictionaryType;
import com.mlt.metadata.stream.LengthType;
import com.mlt.metadata.stream.StreamMetadataDecoder;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.*;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.tuple.Triple;

public class StringDecoder {

  private StringDecoder() {}

  public static HashMap<String, List<String>> resolve(
      Triple<HashMap<String, Integer>, HashMap<String, BitSet>, Map<String, List<String>>> result) {
    var numValues = result.getLeft();
    var presentStreams = result.getMiddle();
    var propertyColumns = result.getRight();
    var propertyMap = new HashMap<String, List<String>>();
    for (var propertyColumn : propertyColumns.entrySet()) {
      var columnName = propertyColumn.getKey();
      var columnPresentStream = presentStreams.get(columnName);
      var columnPropertyValues = propertyColumn.getValue();
      var values = new ArrayList<String>();
      var counter = 0;
      for (var j = 0; j < numValues.get(columnName); j++) {
        var value = columnPresentStream.get(j) ? columnPropertyValues.get(counter++) : null;
        values.add(value);
      }
      propertyMap.put(columnName, values);
    }

    return propertyMap;
  }

  public static Triple<HashMap<String, Integer>, HashMap<String, BitSet>, Map<String, List<String>>>
      decodeSharedDictionary(byte[] data, IntWrapper offset, MltTilesetMetadata.Column column)
          throws IOException {
    List<Integer> dictionaryLengthStream = null;
    byte[] dictionaryStream = null;
    List<Integer> symbolLengthStream = null;
    byte[] symbolTableStream = null;

    // TODO: refactor to be spec compliant -> start by decoding the FieldMetadata, StreamMetadata
    // and PresentStream
    boolean dictionaryStreamDecoded = false;
    while (!dictionaryStreamDecoded) {
      var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      switch (streamMetadata.physicalStreamType()) {
        case LENGTH:
          {
            if (LengthType.DICTIONARY.equals(streamMetadata.logicalStreamType().lengthType())) {
              dictionaryLengthStream =
                  IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            } else {
              symbolLengthStream =
                  IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            }
            break;
          }
        case DATA:
          {
            // TODO: fix -> only shared should be allowed in that case
            if (DictionaryType.SINGLE.equals(streamMetadata.logicalStreamType().dictionaryType())
                || DictionaryType.SHARED.equals(
                    streamMetadata.logicalStreamType().dictionaryType())) {
              dictionaryStream =
                  Arrays.copyOfRange(
                      data, offset.get(), offset.get() + streamMetadata.byteLength());
              offset.set(offset.get() + streamMetadata.byteLength());
              dictionaryStreamDecoded = true;
            } else {
              symbolTableStream =
                  Arrays.copyOfRange(
                      data, offset.get(), offset.get() + streamMetadata.byteLength());
              offset.set(offset.get() + streamMetadata.byteLength());
            }
            break;
          }
      }
    }

    List<String> dictionary = null;
    if (symbolTableStream != null) {
      var utf8Values =
          FsstEncoder.decode(
              symbolTableStream,
              symbolLengthStream.stream().mapToInt(i -> i).toArray(),
              dictionaryStream);
      dictionary = decodeDictionary(dictionaryLengthStream, utf8Values);
    } else {
      dictionary = decodeDictionary(dictionaryLengthStream, dictionaryStream);
    }

    var presentStreams = new HashMap<String, BitSet>();
    var numValues = new HashMap<String, Integer>();
    var values = new HashMap<String, List<String>>();
    for (var childField : column.getComplexType().getChildrenList()) {
      var numStreams = DecodingUtils.decodeVarint(data, offset, 1)[0];
      if (numStreams != 2
          || childField.hasComplexField()
          || childField.getScalarField().getPhysicalType()
              != MltTilesetMetadata.ScalarType.STRING) {
        throw new IllegalArgumentException(
            "Currently only optional string fields are implemented for a struct.");
      }

      var presentStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      var presentStream =
          DecodingUtils.decodeBooleanRle(
              data, presentStreamMetadata.numValues(), presentStreamMetadata.byteLength(), offset);
      var dataStreamMetadata = StreamMetadataDecoder.decode(data, offset);
      var dataReferenceStream =
          IntegerDecoder.decodeIntStream(data, offset, dataStreamMetadata, false);

      var propertyValues = new ArrayList<String>(presentStreamMetadata.numValues());
      var counter = 0;
      for (var i = 0; i < presentStreamMetadata.numValues(); i++) {
        var present = presentStream.get(i);
        if (present) {
          var dataReference = dataReferenceStream.get(counter++);
          var value = dictionary.get(dataReference);
          propertyValues.add(value);
        } else {
          propertyValues.add(null);
        }
      }

      // TODO: get delimiter sign from column mappings
      var columnName =
          column.getName()
              + (childField.getName() == "default" ? "" : (":" + childField.getName()));
      // TODO: refactor to work also when present stream is null
      numValues.put(columnName, presentStreamMetadata.numValues());
      presentStreams.put(columnName, presentStream);
      values.put(columnName, propertyValues);
    }

    return Triple.of(numValues, presentStreams, values);
  }

  private static List<String> decodeDictionary(List<Integer> lengthStream, byte[] utf8Values) {
    // var strValues = new String(utf8Values, StandardCharsets.UTF_8);
    var dictionary = new ArrayList<String>();
    var dictionaryOffset = 0;
    for (var length : lengthStream) {
      // var value = strValues.substring(dictionaryOffset, dictionaryOffset + length);
      var value = Arrays.copyOfRange(utf8Values, dictionaryOffset, dictionaryOffset + length);
      dictionary.add(new String(value, StandardCharsets.UTF_8));
      dictionaryOffset += length;
    }

    return dictionary;
  }

  public static Triple<Integer, BitSet, List<String>> decode(
      byte[] data, IntWrapper offset, int numStreams, BitSet presentStream, int numValues)
      throws IOException {
    /*
     * String column layouts:
     * -> plain -> present, length, data
     * -> dictionary -> present, length, dictionary, data
     * -> fsst dictionary -> symbolTable, symbolLength, dictionary, length, present, data
     * */

    // BitSet presentStream = null;
    List<Integer> dictionaryLengthStream = null;
    List<Integer> offsetStream = null;
    byte[] dataStream = null;
    byte[] dictionaryStream = null;
    List<Integer> symbolLengthStream = null;
    byte[] symbolTableStream = null;
    for (var i = 0; i < numStreams; i++) {
      var streamMetadata = StreamMetadataDecoder.decode(data, offset);
      switch (streamMetadata.physicalStreamType()) {
          /*case PRESENT: {
              //var presentStreamMetadata = StreamMetadata.decode(data, offset);
              //TODO: set numValues in different stream if present stream is nullable
              numValues = streamMetadata.numValues();
              presentStream = DecodingUtils.decodeBooleanRle(data, streamMetadata.numValues(), streamMetadata.byteLength(), offset);
              break;
          }*/
        case OFFSET:
          {
            offsetStream = IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            break;
          }
        case LENGTH:
          {
            var ls = IntegerDecoder.decodeIntStream(data, offset, streamMetadata, false);
            if (LengthType.DICTIONARY.equals(streamMetadata.logicalStreamType().lengthType())) {
              dictionaryLengthStream = ls;
            } else {
              symbolLengthStream = ls;
            }

            break;
          }
        case DATA:
          {
            var ds =
                Arrays.copyOfRange(data, offset.get(), offset.get() + streamMetadata.byteLength());
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
      var utf8Values =
          FsstEncoder.decode(
              symbolTableStream,
              symbolLengthStream.stream().mapToInt(i -> i).toArray(),
              dictionaryStream);
      return Triple.of(
          numValues,
          presentStream,
          decodeDictionary(
              presentStream, dictionaryLengthStream, utf8Values, offsetStream, numValues));
    } else if (dictionaryStream != null) {
      return Triple.of(
          numValues,
          presentStream,
          decodeDictionary(
              presentStream, dictionaryLengthStream, dictionaryStream, offsetStream, numValues));
    } else {
      return Triple.of(
          numValues,
          presentStream,
          decodePlain(presentStream, dictionaryLengthStream, dataStream, numValues));
    }
  }

  private static List<String> decodePlain(
      BitSet presentStream, List<Integer> lengthStream, byte[] utf8Values, int numValues) {
    var decodedValues = new ArrayList<String>(numValues);
    var lengthOffset = 0;
    var strOffset = 0;
    for (var i = 0; i < numValues; i++) {
      var present = presentStream.get(i);
      if (present) {
        var length = lengthStream.get(lengthOffset++);
        var value = new String(utf8Values, strOffset, strOffset + length, StandardCharsets.UTF_8);
        decodedValues.add(value);
        strOffset += length;
      } else {
        decodedValues.add(null);
      }
    }

    return decodedValues;
  }

  private static List<String> decodeDictionary(
      BitSet presentStream,
      List<Integer> lengthStream,
      byte[] utf8Values,
      List<Integer> dictionaryOffsets,
      int numValues) {
    var dictionary = new ArrayList<String>();
    var dictionaryOffset = 0;
    for (var length : lengthStream) {
      var value =
          new String(
              Arrays.copyOfRange(utf8Values, dictionaryOffset, dictionaryOffset + length),
              StandardCharsets.UTF_8);
      dictionary.add(value);
      dictionaryOffset += length;
    }

    var values = new ArrayList<String>(numValues);
    var offset = 0;
    for (var i = 0; i < numValues; i++) {
      var present = presentStream.get(i);
      if (present) {
        var value = dictionary.get(dictionaryOffsets.get(offset++));
        values.add(value);
      } else {
        values.add(null);
      }
    }

    return values;
  }

  public static List<String> decodeFsstDictionaryEncodedStringColumn(byte[] data, IntWrapper offset)
      throws IOException {
    /* FsstDictionary -> SymbolTable, SymbolLength, CompressedCorups, Length, Data */
    // TODO: get rid of that IntWrapper creation
    var symbolTableOffset = new IntWrapper(offset.get());
    var symbolTableMetadata = StreamMetadataDecoder.decode(data, symbolTableOffset);
    var symbolLengthOffset =
        new IntWrapper(symbolTableOffset.get() + symbolTableMetadata.byteLength());
    var symbolLengthMetadata = StreamMetadataDecoder.decode(data, symbolLengthOffset);
    var compressedCorpusOffset =
        new IntWrapper(symbolLengthOffset.get() + symbolLengthMetadata.byteLength());
    var compressedCorpusMetadata = StreamMetadataDecoder.decode(data, compressedCorpusOffset);
    var lengthOffset =
        new IntWrapper(compressedCorpusOffset.get() + compressedCorpusMetadata.byteLength());
    var lengthMetadata = StreamMetadataDecoder.decode(data, lengthOffset);
    var dataOffset = new IntWrapper(lengthOffset.get() + lengthMetadata.byteLength());
    var dataMetadata = StreamMetadataDecoder.decode(data, dataOffset);

    // TODO: get rid of that copy by refactoring the fsst decoding function
    var symbols =
        Arrays.copyOfRange(
            data,
            symbolTableOffset.get(),
            symbolTableOffset.get() + symbolTableMetadata.byteLength());
    var symbolLength =
        IntegerDecoder.decodeIntStream(data, symbolLengthOffset, symbolLengthMetadata, false);
    var compressedCorpus =
        Arrays.copyOfRange(
            data,
            compressedCorpusOffset.get(),
            compressedCorpusOffset.get() + compressedCorpusMetadata.byteLength());
    var values =
        FsstEncoder.decode(
            symbols, symbolLength.stream().mapToInt(i -> i).toArray(), compressedCorpus);

    var length = IntegerDecoder.decodeIntStream(data, lengthOffset, lengthMetadata, false);
    var decodedData = IntegerDecoder.decodeIntStream(data, dataOffset, dataMetadata, false);

    var decodedDictionary = new ArrayList<String>();
    var strStart = 0;
    for (var l : length) {
      var v = Arrays.copyOfRange(values, strStart, strStart + l);
      decodedDictionary.add(new String(v, StandardCharsets.UTF_8));
      strStart += l;
    }

    var decodedValues = new ArrayList<String>(decodedData.size());
    for (var dictionaryOffset : decodedData) {
      var value = decodedDictionary.get(dictionaryOffset);
      decodedValues.add(value);
    }

    // TODO: check -> is this correct?
    offset.set(dataOffset.get());

    return decodedValues;
  }
}
