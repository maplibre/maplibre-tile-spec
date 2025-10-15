package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.stream.Collectors;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.encodings.fsst.FsstEncoder;
import org.maplibre.mlt.metadata.stream.*;

public class StringEncoder {

  private StringEncoder() {}

  public static Pair<Integer, byte[]> encodeSharedDictionary(
      List<List<String>> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFsstEncoding,
      @NotNull MLTStreamObserver streamRecorder,
      @Nullable String fieldName)
      throws IOException {
    /*
     * compare single column encoding with shared dictionary encoding
     * Shared dictionary layout -> length, dictionary, present1, data1, present2, data2
     * Shared Fsst dictionary layout ->  symbol table, symbol length, dictionary/compressed corpus, length, present1, data1, present2, data2
     * */
    // TODO: also compare size with plain and single encoded columns
    // var lengthStream = new ArrayList<Integer>();
    // TODO: also sort dictionary for the usage in combination with Gzip?
    var dictionary = new ArrayList<String>();
    var dataStreams = new ArrayList<List<Integer>>(values.size());
    var presentStreams = new ArrayList<List<Boolean>>();
    for (var column : values) {
      var presentStream = new ArrayList<Boolean>();
      presentStreams.add(presentStream);
      var dataStream = new ArrayList<Integer>();
      dataStreams.add(dataStream);
      for (var value : column) {
        if (value == null) {
          presentStream.add(false);
        } else {
          presentStream.add(true);

          if (!dictionary.contains(value)) {
            dictionary.add(value);
            var index = dictionary.size() - 1;
            dataStream.add(index);
          } else {
            var index = dictionary.indexOf(value);
            dataStream.add(index);
          }
        }
      }
    }

    if (dictionary.isEmpty()) {
      /* Set number of streams to zero if no columns are present in this tile */
      return Pair.of(0, new byte[0]);
    }

    var encodedSharedDictionary =
        encodeDictionary(
            dictionary, physicalLevelTechnique, false, true, streamRecorder, fieldName + "_dict");

    byte[] encodedSharedFsstDictionary = null;
    if (useFsstEncoding) {
      encodedSharedFsstDictionary =
          encodeFsstDictionary(
              dictionary, physicalLevelTechnique, false, true, streamRecorder, fieldName + "_fsst");
    }

    var sharedDictionary =
        encodedSharedFsstDictionary != null
                && encodedSharedFsstDictionary.length < encodedSharedDictionary.length
            ? encodedSharedFsstDictionary
            : encodedSharedDictionary;

    for (var i = 0; i < dataStreams.size(); i++) {
      var presentStream = presentStreams.get(i);
      var dataStream = dataStreams.get(i);

      if (dataStream.isEmpty()) {
        /* If no values are present in this column add zero for number of streams */
        final var encodedFieldMetadata = EncodingUtils.encodeVarint(0, false);
        sharedDictionary = CollectionUtils.concatByteArrays(sharedDictionary, encodedFieldMetadata);
        continue;
      }

      final var encodedFieldMetadata = EncodingUtils.encodeVarints(new int[] {2}, false, false);
      final var encodedPresentStream =
          BooleanEncoder.encodeBooleanStream(
              presentStream,
              PhysicalStreamType.PRESENT,
              streamRecorder,
              fieldName + "_present_" + i);

      final var encodedDataStream =
          IntegerEncoder.encodeIntStream(
              dataStream,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.STRING),
              streamRecorder,
              fieldName + "_" + i);
      sharedDictionary =
          CollectionUtils.concatByteArrays(
              sharedDictionary, encodedFieldMetadata, encodedPresentStream, encodedDataStream);
    }

    // TODO: make present stream optional
    final var numStreams =
        (encodedSharedFsstDictionary != null
                    && encodedSharedFsstDictionary.length < encodedSharedDictionary.length
                ? 5
                : 3)
            + values.size() * 2;
    return Pair.of(numStreams, sharedDictionary);
  }

  public static Pair<Integer, byte[]> encode(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFsstEncoding,
      @NotNull MLTStreamObserver streamRecorder,
      @Nullable String fieldName)
      throws IOException {
    /*
     * convert a single string column -> check if plain, dictionary, fsst or fsstDictionary
     * -> plain -> length, data
     * -> dictionary -> length, dictionary, data
     * -> fsst -> symbol length, symbol table, length, compressed corpus
     * -> fsst dictionary -> symbol length, symbol table,  length, dictionary/compressed corpus, offset
     * Schema selection
     * -> based on statistics if dictionary encoding is used
     * -> compare four possible encodings in size based on samples
     * */
    // TODO: add plain encoding again
    // var plainEncodedColumn = encodePlain(values, physicalLevelTechnique);
    var dictionaryEncodedColumn =
        encodeDictionary(values, physicalLevelTechnique, true, false, streamRecorder, fieldName);
    if (!useFsstEncoding) {
      return Pair.of(3, dictionaryEncodedColumn);
    }

    var fsstEncodedDictionary =
        encodeFsstDictionary(
            values, physicalLevelTechnique, true, false, streamRecorder, fieldName);
    return dictionaryEncodedColumn.length <= fsstEncodedDictionary.length
        ? Pair.of(3, dictionaryEncodedColumn)
        : Pair.of(5, fsstEncodedDictionary);
  }

  private static byte[] encodeFsstDictionary(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean encodeDataStream,
      boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamRecorder,
      String fieldName)
      throws IOException {
    var dataStream = new ArrayList<Integer>(values.size());
    // var lengthStream = new ArrayList<Integer>();
    var dictionary = new ArrayList<String>();
    for (var value : values) {
      if (!dictionary.contains(value)) {
        dictionary.add(value);
        var utf8EncodedData = value.getBytes(StandardCharsets.UTF_8);
        // lengthStream.add(utf8EncodedData.length);
        var index = dictionary.size() - 1;
        dataStream.add(index);
      } else {
        var index = dictionary.indexOf(value);
        dataStream.add(index);
      }
    }

    var symbolTable =
        encodeFsst(dictionary, physicalLevelTechnique, false, streamRecorder, fieldName);

    if (!encodeDataStream) {
      return symbolTable;
    }

    var encodedDataStream =
        IntegerEncoder.encodeIntStream(
            dataStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.STRING),
            streamRecorder,
            fieldName);
    return CollectionUtils.concatByteArrays(symbolTable, encodedDataStream);
  }

  private static byte[] encodeFsst(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @SuppressWarnings("SameParameterValue") boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamRecorder,
      @Nullable String fieldName)
      throws IOException {
    var joinedValues = String.join("", values).getBytes(StandardCharsets.UTF_8);
    var symbolTable = FsstEncoder.encode(joinedValues);

    var encodedSymbols = symbolTable.symbols();
    var compressedCorpus = symbolTable.compressedData();

    var symbolLengths =
        Arrays.stream(symbolTable.symbolLengths()).boxed().collect(Collectors.toList());
    var encodedSymbolLengths =
        IntegerEncoder.encodeIntStream(
            symbolLengths,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.SYMBOL),
            streamRecorder,
            fieldName + "_symlength");
    var symbolTableMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(DictionaryType.FSST),
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                // TODO: numValues in this context not needed -> set 0 to save space -> only 1 byte
                // with varint?
                symbolLengths.size(),
                encodedSymbols.length)
            .encode();
    var lengthStream =
        values.stream()
            .map(v -> v.getBytes(StandardCharsets.UTF_8).length)
            .collect(Collectors.toList());
    var encodedLengthStream =
        IntegerEncoder.encodeIntStream(
            lengthStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.DICTIONARY),
            streamRecorder,
            fieldName + "_length");
    var compressedCorpusStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(
                    isSharedDictionary ? DictionaryType.SHARED : DictionaryType.SINGLE),
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                compressedCorpus.length)
            .encode();

    streamRecorder.observeStream(
        fieldName + "_corpus", values, compressedCorpusStreamMetadata, compressedCorpus);

    // TODO: how to name the streams and how to order? -> symbol_table, length, data, length
    /* SymbolLength, SymbolTable, Value Length, Compressed Corpus */
    return CollectionUtils.concatByteArrays(
        encodedSymbolLengths,
        symbolTableMetadata,
        symbolTable.symbols(),
        encodedLengthStream,
        compressedCorpusStreamMetadata,
        compressedCorpus);
  }

  private static byte[] encodeDictionary(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean encodeOffsetStream,
      boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamRecorder,
      String fieldName)
      throws IOException {
    var offsetStream = new ArrayList<Integer>(values.size());
    var lengthStream = new ArrayList<Integer>();
    var dictionary = new ArrayList<String>();
    var dictionaryStream = new byte[0];
    for (var value : values) {
      if (!dictionary.contains(value)) {
        dictionary.add(value);
        var utf8EncodedData = value.getBytes(StandardCharsets.UTF_8);
        lengthStream.add(utf8EncodedData.length);
        var index = dictionary.size() - 1;
        offsetStream.add(index);

        dictionaryStream = CollectionUtils.concatByteArrays(dictionaryStream, utf8EncodedData);
      } else {
        var index = dictionary.indexOf(value);
        offsetStream.add(index);
      }
    }

    var encodedLengthStream =
        IntegerEncoder.encodeIntStream(
            lengthStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.DICTIONARY),
            streamRecorder,
            fieldName + "_dict_length");
    var encodedDictionaryStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(
                    isSharedDictionary ? DictionaryType.SHARED : DictionaryType.SINGLE),
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                dictionary.size(),
                dictionaryStream.length)
            .encode();

    streamRecorder.observeStream(
        fieldName + "_dict", dictionary, encodedDictionaryStreamMetadata, dictionaryStream);
    if (!encodeOffsetStream) {
      return CollectionUtils.concatByteArrays(
          encodedLengthStream, encodedDictionaryStreamMetadata, dictionaryStream);
    }

    var encodedOffsetStream =
        IntegerEncoder.encodeIntStream(
            offsetStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.STRING),
            streamRecorder,
            fieldName + "_dict_offset");
    /* Length, Offset (String), Data (Dictionary -> Single) */
    return CollectionUtils.concatByteArrays(
        encodedLengthStream,
        encodedOffsetStream,
        encodedDictionaryStreamMetadata,
        dictionaryStream);
  }

  public static byte[] encodePlain(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull MLTStreamObserver streamRecorder,
      @Nullable String fieldName)
      throws IOException {
    var lengthStream = new ArrayList<Integer>(values.size());
    var dataStream = new byte[0];
    for (var value : values) {
      var utf8EncodedValue = value.getBytes(StandardCharsets.UTF_8);
      dataStream = CollectionUtils.concatByteArrays(dataStream, utf8EncodedValue);
      lengthStream.add(utf8EncodedValue.length);
    }

    var encodedLengthStream =
        IntegerEncoder.encodeIntStream(
            lengthStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.VAR_BINARY),
            streamRecorder,
            fieldName + "_length");

    var dataStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(DictionaryType.NONE),
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                dataStream.length)
            .encode();

    streamRecorder.observeStream(fieldName + "_data", values, dataStreamMetadata, dataStream);

    return CollectionUtils.concatByteArrays(encodedLengthStream, dataStreamMetadata, dataStream);
  }
}
