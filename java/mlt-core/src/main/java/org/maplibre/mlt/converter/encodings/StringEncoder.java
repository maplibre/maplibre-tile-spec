package org.maplibre.mlt.converter.encodings;

import jakarta.annotation.Nullable;
import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.UncheckedIOException;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collection;
import java.util.Comparator;
import java.util.HashMap;
import java.util.List;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.apache.commons.lang3.mutable.MutableInt;
import org.apache.commons.lang3.tuple.Pair;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.MLTStreamObserver;
import org.maplibre.mlt.converter.encodings.fsst.FsstEncoder;
import org.maplibre.mlt.metadata.stream.DictionaryType;
import org.maplibre.mlt.metadata.stream.LengthType;
import org.maplibre.mlt.metadata.stream.LogicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.LogicalStreamType;
import org.maplibre.mlt.metadata.stream.OffsetType;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.stream.PhysicalStreamType;
import org.maplibre.mlt.metadata.stream.StreamMetadata;
import org.maplibre.mlt.util.ByteBufferUtil;

public class StringEncoder {

  private StringEncoder() {}

  /// Convert a collection of string columns into a shared dictionary encoded byte array
  /// @return Pair of (number of streams, dictionary encoded to byte array)
  public static Pair<Integer, List<ByteBuffer>> encodeSharedDictionary(
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
    // TODO: also sort dictionary for the usage in combination with Gzip?
    final var dictionary = new ArrayList<String>(values.size());
    final var dictionarySize = new MutableInt(0);
    final var dictMap = new HashMap<String, Integer>(values.size());
    final var dataStreams = new ArrayList<List<Integer>>(values.size());
    final var presentStreams = new ArrayList<Boolean[]>(values.size());
    for (var column : values) {
      final var presentStream = new Boolean[column.size()];
      final var dataStream = new ArrayList<Integer>(column.size());
      presentStreams.add(presentStream);
      dataStreams.add(dataStream);

      var presentIndex = 0;
      for (var value : column) {
        presentStream[presentIndex++] = (value != null);
        if (value != null) {
          dataStream.add(
              dictMap.computeIfAbsent(
                  value,
                  v -> {
                    final var index = dictionarySize.getAndIncrement();
                    dictionary.add(v);
                    return index;
                  }));
        }
      }
    }

    if (dictionarySize.intValue() == 0) {
      /* Set number of streams to zero if no columns are present in this tile */
      return Pair.of(0, List.of());
    }

    final var encodedSharedDictionary =
        encodeDictionary(
            dictionary, physicalLevelTechnique, false, true, streamObserver, fieldName);

    final List<ByteBuffer> encodedSharedFsstDictionary;
    if (useFsstEncoding) {
      encodedSharedFsstDictionary =
          encodeFsstDictionary(
              dictionary,
              dictionarySize.intValue(),
              physicalLevelTechnique,
              false,
              true,
              streamObserver,
              fieldName + "_fsst");
    } else {
      encodedSharedFsstDictionary = null;
    }

    final var usingFSST =
        useFsstEncoding
            && ByteBufferUtil.totalLength(encodedSharedFsstDictionary)
                < ByteBufferUtil.totalLength(encodedSharedDictionary);
    final var result = usingFSST ? encodedSharedFsstDictionary : encodedSharedDictionary;

    final var width = (int) Math.max(1, Math.ceil(Math.log10(dataStreams.size() - 1)));
    for (var i = 0; i < dataStreams.size(); i++) {
      var presentStream = presentStreams.get(i);
      var dataStream = dataStreams.get(i);

      if (dataStream.isEmpty()) {
        /* If no values are present in this column add zero for number of streams */
        final var encodedFieldMetadata = EncodingUtils.encodeVarint(0, false);
        result.add(encodedFieldMetadata);
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
      result.add(encodedFieldMetadata);
      result.addAll(encodedPresentStream);
      result.addAll(encodedDataStream);
    }

    // TODO: make present stream optional
    final var numStreams = (usingFSST ? 5 : 3) + values.size() * 2;
    return Pair.of(numStreams, result);
  }

  public static Pair<Integer, List<ByteBuffer>> encode(
      Collection<String> values,
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

    final List<ByteBuffer> fsstEncodedDictionary;
    if (useFsstEncoding) {
      fsstEncodedDictionary =
          encodeFsstDictionary(
              values,
              values.size(),
              physicalLevelTechnique,
              true,
              false,
              streamObserver,
              fieldName + "_fsstdict");
    } else {
      fsstEncodedDictionary = null;
    }

    return Stream.of(
            Pair.of(2, plainEncodedColumn),
            Pair.of(3, dictionaryEncodedColumn),
            Pair.of(5, fsstEncodedDictionary))
        .filter(p -> p.getRight() != null)
        .min(Comparator.comparingInt(a -> ByteBufferUtil.totalLength(a.getRight())))
        .orElseThrow();
  }

  private static List<ByteBuffer> encodeFsstDictionary(
      Collection<String> values,
      int valueCount,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean encodeDataStream,
      boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamObserver,
      String fieldName)
      throws IOException {
    final var dataStream = new ArrayList<Integer>(valueCount);
    final var dictionary = new ArrayList<String>(valueCount);
    final var dictMap = new HashMap<String, Integer>(valueCount);
    for (var value : values) {
      dataStream.add(
          dictMap.computeIfAbsent(
              value,
              v -> {
                dictionary.add(v);
                return dictionary.size() - 1;
              }));
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
    symbolTable.addAll(encodedDataStream);
    return symbolTable;
  }

  private static List<ByteBuffer> encodeFsst(
      Collection<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @SuppressWarnings("SameParameterValue") boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String fieldName)
      throws IOException {
    final var joinedValues = String.join("", values).getBytes(StandardCharsets.UTF_8);
    final var symbolTable = FsstEncoder.encode(joinedValues);

    final var encodedSymbols = symbolTable.symbols();
    final var compressedCorpus = symbolTable.compressedData();

    final var symbolLengths =
        Arrays.stream(symbolTable.symbolLengths()).boxed().collect(Collectors.toList());
    final var encodedSymbolLengths =
        IntegerEncoder.encodeIntStream(
            symbolLengths,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.SYMBOL),
            streamObserver,
            fieldName + "_symlength");
    final var symbolTableMetadata =
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
    final var lengthStream =
        values.stream().mapToInt(v -> v.getBytes(StandardCharsets.UTF_8).length).toArray();
    final var encodedLengthStream =
        IntegerEncoder.encodeIntStream(
            lengthStream,
            physicalLevelTechnique,
            false,
            PhysicalStreamType.LENGTH,
            new LogicalStreamType(LengthType.DICTIONARY),
            streamObserver,
            fieldName + "_length");
    final var compressedCorpusStreamMetadata =
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

    /* SymbolLength, SymbolTable, Value Length, Compressed Corpus */
    final var result = encodedSymbolLengths;
    result.addAll(symbolTableMetadata);
    result.add(ByteBuffer.wrap(symbolTable.symbols()));
    result.addAll(encodedLengthStream);
    result.addAll(compressedCorpusStreamMetadata);
    result.add(ByteBuffer.wrap(compressedCorpus));
    return result;
  }

  private static int totalLengthOf(Collection<String> values) {
    return values.stream().mapToInt(String::length).sum();
  }

  private static List<ByteBuffer> encodeDictionary(
      Collection<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean encodeOffsetStream,
      boolean isSharedDictionary,
      @NotNull MLTStreamObserver streamObserver,
      String fieldName)
      throws IOException {
    final var offsetStream = new ArrayList<Integer>(values.size());
    final var lengthStream = new ArrayList<Integer>(values.size());
    final var dictionary = new ArrayList<String>(values.size());
    final var dictMap = new HashMap<String, Integer>(values.size());
    final var dictionaryStream = new ByteArrayOutputStream(totalLengthOf(values));
    for (var value : values) {
      offsetStream.add(
          dictMap.computeIfAbsent(
              value,
              v -> {
                dictionary.add(v);
                final var utf8EncodedData = value.getBytes(StandardCharsets.UTF_8);
                lengthStream.add(utf8EncodedData.length);
                try {
                  dictionaryStream.write(utf8EncodedData);
                } catch (IOException ex) {
                  throw new UncheckedIOException(ex);
                }
                return dictionary.size() - 1;
              }));
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
        fieldName + "_dict", dictionary.toArray(), encodedDictionaryStreamMetadata, dictionaryData);
    if (!encodeOffsetStream) {
      final var result = encodedLengthStream;
      result.addAll(encodedDictionaryStreamMetadata);
      result.add(ByteBuffer.wrap(dictionaryData));
      return result;
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
    final var result = encodedLengthStream;
    result.addAll(encodedOffsetStream);
    result.addAll(encodedDictionaryStreamMetadata);
    result.add(ByteBuffer.wrap(dictionaryData));
    return result;
  }

  public static List<ByteBuffer> encodePlain(
      Collection<String> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      @NotNull MLTStreamObserver streamObserver,
      @Nullable String fieldName)
      throws IOException {
    final var lengthStream = new ArrayList<Integer>(values.size());
    final var dataStream = new ByteArrayOutputStream(totalLengthOf(values));
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

    final var result = encodedLengthStream;
    result.addAll(dataStreamMetadata);
    result.add(ByteBuffer.wrap(stringData));
    return result;
  }
}
