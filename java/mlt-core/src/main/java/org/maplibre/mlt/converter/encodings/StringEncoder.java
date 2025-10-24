package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Comparator;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.CollectionUtils;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.encodings.fsst.FsstEncoder;
import org.maplibre.mlt.metadata.stream.*;

public class StringEncoder {

  private StringEncoder() {}

  /// Convert a collection of string columns into a shared dictionary encoded byte array
  /// @return Pair of (number of streams, dictionary encoded to byte array)
  public static Pair<Integer, byte[]> encodeSharedDictionary(
      List<List<String>> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFsstEncoding,
      @NotNull MLTStreamObserver streamObserver,
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
            dictionary, physicalLevelTechnique, false, true, streamObserver, fieldName);

    byte[] encodedSharedFsstDictionary = null;
    if (useFsstEncoding) {
      encodedSharedFsstDictionary =
          encodeFsstDictionary(
              dictionary, physicalLevelTechnique, false, true, streamObserver, fieldName + "_fsst");
    }

    var sharedDictionary =
        encodedSharedFsstDictionary != null
                && encodedSharedFsstDictionary.length < encodedSharedDictionary.length
            ? encodedSharedFsstDictionary
            : encodedSharedDictionary;

    final var width = (int) Math.max(1, Math.ceil(Math.log10(dataStreams.size() - 1)));
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
      final var fieldNumber = String.format("%0" + width + "d", i);
      final var encodedPresentStream =
          BooleanEncoder.encodeBooleanStream(
              presentStream,
              PhysicalStreamType.PRESENT,
              streamObserver,
              fieldName + "_child" + fieldNumber + "_present");

      final var encodedDataStream =
          IntegerEncoder.encodeIntStream(
              dataStream,
              physicalLevelTechnique,
              false,
              PhysicalStreamType.OFFSET,
              new LogicalStreamType(OffsetType.STRING),
              streamObserver,
              fieldName + "_child" + fieldNumber + "_offset");
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
      @NotNull MLTStreamObserver streamObserver,
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
    final var plainEncodedColumn =
        encodePlain(values, physicalLevelTechnique, streamObserver, fieldName + "_plain");

    final var dictionaryEncodedColumn =
        encodeDictionary(
            values, physicalLevelTechnique, true, false, streamObserver, fieldName + "_dict");

    byte[] fsstEncodedDictionary = null;
    if (useFsstEncoding) {
      fsstEncodedDictionary =
          encodeFsstDictionary(
              values, physicalLevelTechnique, true, false, streamObserver, fieldName + "_fsstdict");
    }

    return Stream.of(
            Pair.of(2, plainEncodedColumn),
            Pair.of(3, dictionaryEncodedColumn),
            Pair.of(5, fsstEncodedDictionary))
        .filter(p -> p.getRight() != null)
        .min(Comparator.comparingInt(a -> a.getRight().length))
        .orElseThrow();
  }

  private static byte[] encodeFsstDictionary(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean encodeDataStream,
      boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamObserver,
      String fieldName)
      throws IOException {
    final var dataStream = new ArrayList<Integer>(values.size());
    final var dictionary = new ArrayList<String>();
    for (var value : values) {
      if (!dictionary.contains(value)) {
        dictionary.add(value);
        dataStream.add(dictionary.size() - 1);
      } else {
        dataStream.add(dictionary.indexOf(value));
      }
    }

    final var symbolTable =
        encodeFsst(dictionary, physicalLevelTechnique, false, streamObserver, fieldName);

    if (!encodeDataStream) {
      return symbolTable;
    }

    final var encodedDataStream =
        IntegerEncoder.encodeIntStream(
            dataStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.STRING),
            streamObserver,
            fieldName);
    return CollectionUtils.concatByteArrays(symbolTable, encodedDataStream);
  }

  private static byte[] encodeFsst(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @SuppressWarnings("SameParameterValue") boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamObserver,
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
            streamObserver,
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
            streamObserver,
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

    streamObserver.observeStream(
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
      @NotNull MLTStreamObserver streamObserver,
      String fieldName)
      throws IOException {
    final var offsetStream = new ArrayList<Integer>(values.size());
    final var lengthStream = new ArrayList<Integer>();
    final var dictionary = new ArrayList<String>();
    final var dictionaryStream = new ByteArrayOutputStream();
    for (var value : values) {
      if (!dictionary.contains(value)) {
        dictionary.add(value);
        var utf8EncodedData = value.getBytes(StandardCharsets.UTF_8);
        lengthStream.add(utf8EncodedData.length);
        var index = dictionary.size() - 1;
        offsetStream.add(index);

        dictionaryStream.write(utf8EncodedData);
      } else {
        var index = dictionary.indexOf(value);
        offsetStream.add(index);
      }
    }
    final var dictionaryData = dictionaryStream.toByteArray();

    final var encodedLengthStream =
        IntegerEncoder.encodeIntStream(
            lengthStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.DICTIONARY),
            streamObserver,
            fieldName + "_dict_length");
    final var encodedDictionaryStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(
                    isSharedDictionary ? DictionaryType.SHARED : DictionaryType.SINGLE),
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                dictionary.size(),
                dictionaryData.length)
            .encode();

    streamObserver.observeStream(
        fieldName + "_dict", dictionary, encodedDictionaryStreamMetadata, dictionaryData);
    if (!encodeOffsetStream) {
      return CollectionUtils.concatByteArrays(
          encodedLengthStream, encodedDictionaryStreamMetadata, dictionaryData);
    }

    final var encodedOffsetStream =
        IntegerEncoder.encodeIntStream(
            offsetStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.OFFSET,
            new LogicalStreamType(OffsetType.STRING),
            streamObserver,
            fieldName + "_dict_offset");
    /* Length, Offset (String), Data (Dictionary -> Single) */
    return CollectionUtils.concatByteArrays(
        encodedLengthStream, encodedOffsetStream, encodedDictionaryStreamMetadata, dictionaryData);
  }

  public static byte[] encodePlain(
      List<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String fieldName)
      throws IOException {
    final var lengthStream = new ArrayList<Integer>(values.size());
    final var dataStream = new ByteArrayOutputStream();
    for (var value : values) {
      final var utf8EncodedValue = value.getBytes(StandardCharsets.UTF_8);
      dataStream.write(utf8EncodedValue);
      lengthStream.add(utf8EncodedValue.length);
    }
    final var stringData = dataStream.toByteArray();

    final var encodedLengthStream =
        IntegerEncoder.encodeIntStream(
            lengthStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.VAR_BINARY),
            streamObserver,
            fieldName + "_length");

    final var dataStreamMetadata =
        new StreamMetadata(
                PhysicalStreamType.DATA,
                new LogicalStreamType(DictionaryType.NONE),
                LogicalLevelTechnique.NONE,
                LogicalLevelTechnique.NONE,
                PhysicalLevelTechnique.NONE,
                values.size(),
                stringData.length)
            .encode();

    streamObserver.observeStream(fieldName + "_data", values, dataStreamMetadata, stringData);

    return CollectionUtils.concatByteArrays(encodedLengthStream, dataStreamMetadata, stringData);
  }
}
