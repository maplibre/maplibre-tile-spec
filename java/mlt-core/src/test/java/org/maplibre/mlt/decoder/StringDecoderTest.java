package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.function.Function;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.tuple.Pair;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;
import org.junit.jupiter.params.provider.ValueSource;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.converter.MLTStreamObserverDefault;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.encodings.StringEncoder;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class StringDecoderTest {

  public static Pair<Integer, byte[]> encodeSharedDictionary(
      List<List<String>> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFsstEncoding)
      throws IOException {
    return StringEncoder.encodeSharedDictionary(
        values, physicalLevelTechnique, useFsstEncoding, new MLTStreamObserverDefault(), null);
  }

  @Test
  @Disabled
  public void decodeSharedDictionary_FsstDictionaryEncoded() throws IOException {
    var values1 =
        List.of(
            "TestTestTestTestTestTest",
            "TestTestTestTestTestTest1",
            "TestTestTestTestTestTest2",
            "TestTestTestTestTestTest2",
            "TestTestTestTestTestTest4");
    var values2 =
        List.of(
            "TestTestTestTestTestTest6",
            "TestTestTestTestTestTest5",
            "TestTestTestTestTestTest8",
            "TestTestTestTestTestTes9",
            "TestTestTestTestTestTest10");
    var values = List.of(values1, values2);
    var encodedValues = encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, true);

    var tileMetadata =
        MltTilesetMetadata.Column.newBuilder()
            .setName("Test")
            .setNullable(true)
            .setScalarType(
                MltTilesetMetadata.ScalarColumn.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING))
            .build();

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    var v = decodedValues.getRight();
    Assert.equals(values1, v.get(":Test"));
    Assert.equals(values2, v.get(":Test2"));
  }

  @Test
  @Disabled
  public void decodeSharedDictionary_DictionaryEncoded() throws IOException {
    var values1 = List.of("Test", "Test2", "Test4", "Test2", "Test");
    var values2 = List.of("Test1", "Test2", "Test1", "Test5", "Test");
    var values = List.of(values1, values2);
    var encodedValues = encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, false);

    var test =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var test2 =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test2")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var tileMetadata =
        MltTilesetMetadata.Column.newBuilder()
            .setName("Parent")
            .setNullable(true)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .addChildren(test)
                    .addChildren(test2)
                    .build())
            .build();

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    var v = decodedValues.getRight();
    Assert.equals(values1, v.get(":Test"));
    Assert.equals(values2, v.get(":Test2"));
  }

  @Test
  @Disabled
  public void decodeSharedDictionary_NullValues_DictionaryEncoded() throws IOException {
    var values1 = Arrays.asList("Test", null, "Test2", null, "Test4", "Test2", "Test");
    var values2 =
        Arrays.asList(
            null, "Test1", "Test2", "Test1", null, null, "Test5", null, "Test", null, null);
    var values = List.of(values1, values2);
    var encodedValues = encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, false);

    var test =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var test2 =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test2")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var tileMetadata =
        MltTilesetMetadata.Column.newBuilder()
            .setName("Parent")
            .setNullable(true)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .addChildren(test)
                    .addChildren(test2)
                    .build())
            .build();

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    var v = decodedValues.getRight();

    var actualValues1 = v.get(":Test");
    var p1 = decodedValues.getMiddle().get(":Test");
    var decodedV1 = new ArrayList<String>();
    var counter = 0;
    for (var i = 0; i < decodedValues.getMiddle().size(); i++) {
      var a = p1.get(i) ? actualValues1.get(counter++) : null;
      decodedV1.add(a);
    }
    var actualValues2 = v.get(":Test2");
    var p2 = decodedValues.getMiddle().get(":Test2");
    var decodedV2 = new ArrayList<String>();
    var counter2 = 0;
    for (var i = 0; i < decodedValues.getMiddle().size(); i++) {
      var a = p2.get(i) ? actualValues2.get(counter2++) : null;
      decodedV2.add(a);
    }

    Assert.equals(values1, v.get(":Test"));
    Assert.equals(values2, v.get(":Test2"));
  }

  @Test
  @Disabled
  public void decodeSharedDictionary_NullValues_FsstDictionaryEncoded() throws IOException {
    var values1 =
        Arrays.asList(
            null,
            null,
            null,
            null,
            "TestTestTestTestTestTest",
            "TestTestTestTestTestTest1",
            null,
            "TestTestTestTestTestTest2",
            "TestTestTestTestTestTest2",
            "TestTestTestTestTestTest4");
    var values2 =
        Arrays.asList(
            "TestTestTestTestTestTest6",
            null,
            "TestTestTestTestTestTest5",
            "TestTestTestTestTestTest8",
            "TestTestTestTestTestTes9",
            null,
            "TestTestTestTestTestTest10");
    var values = List.of(values1, values2);
    var encodedValues = encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, true);

    var test =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var test2 =
        MltTilesetMetadata.Field.newBuilder()
            .setName("Test2")
            .setScalarField(
                MltTilesetMetadata.ScalarField.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                    .build());
    var tileMetadata =
        MltTilesetMetadata.Column.newBuilder()
            .setName("Parent")
            .setNullable(true)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .addChildren(test)
                    .addChildren(test2)
                    .build())
            .build();

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    var v = decodedValues.getRight();

    var actualValues1 = v.get(":Test");
    var p1 = decodedValues.getMiddle().get(":Test");
    var decodedV1 = new ArrayList<String>();
    var counter = 0;
    for (var i = 0; i < decodedValues.getMiddle().size(); i++) {
      var a = p1.get(i) ? actualValues1.get(counter++) : null;
      decodedV1.add(a);
    }
    var actualValues2 = v.get(":Test2");
    var p2 = decodedValues.getMiddle().get(":Test2");
    var decodedV2 = new ArrayList<String>();
    var counter2 = 0;
    for (var i = 0; i < decodedValues.getMiddle().size(); i++) {
      var a = p2.get(i) ? actualValues2.get(counter2++) : null;
      decodedV2.add(a);
    }

    Assert.equals(values1, v.get(":Test"));
    Assert.equals(values2, v.get(":Test2"));
  }

  /// Helper method to filter and cast stream elements
  private static <Target extends Base, Base> Function<Base, Stream<Target>> ofType(
      Class<Target> targetType) {
    return value ->
        targetType.isInstance(value) ? Stream.of(targetType.cast(value)) : Stream.empty();
  }

  @ParameterizedTest
  @EnumSource(
      value = PhysicalLevelTechnique.class,
      names = {"VARINT", "FAST_PFOR"})
  public void decodeSharedDictionary_FastPFOR_Mvt(PhysicalLevelTechnique technique)
      throws IOException {
    final var tileId = String.format("%s_%s_%s", 5, 16, 21);
    final var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    final var mvTile = MvtUtils.decodeMvt(mvtFilePath);
    final var values =
        mvTile.layers().getFirst().features().stream()
            .flatMap(
                feature -> feature.properties().values().stream().flatMap(ofType(String.class)))
            .toList();
    final var encodedValues = encodeSharedDictionary(List.of(values), technique, false);
    final var tileMetadata =
        MltTilesetMetadata.Column.newBuilder()
            .setName("TestParent")
            .setNullable(true)
            .setComplexType(
                MltTilesetMetadata.ComplexColumn.newBuilder()
                    .setPhysicalType(MltTilesetMetadata.ComplexType.STRUCT)
                    .addChildren(
                        MltTilesetMetadata.Field.newBuilder()
                            .setName("TestChild")
                            .setScalarField(
                                MltTilesetMetadata.ScalarField.newBuilder()
                                    .setPhysicalType(MltTilesetMetadata.ScalarType.STRING)
                                    .build())))
            .build();
    var decodeResult =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);
    var decodedValues = decodeResult.getRight().get("TestParent:TestChild");
    Assert.equals(values, decodedValues);
  }

  @ParameterizedTest
  @ValueSource(ints = {0, 3})
  public void decodeSharedDictionary_MvtWithNestedColumns(int tableIndex) throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    // Force coverage of the case where the "base" mapped column doesn't appear in the first feature
    mvTile.layers().getFirst().features().getFirst().properties().remove("name");

    final var columnMapping = new ColumnMapping("name", ":", true);
    final var columnMappings = Map.of(Pattern.compile(".*"), List.of(columnMapping));
    final var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);
    final var fieldMetadata =
        tileMetadata.getFeatureTables(tableIndex).getColumnsList().stream()
            .filter(f -> f.getName().equals("name"))
            .findFirst()
            .orElseThrow();

    var layer = mvTile.layers().get(tableIndex);
    var sharedValues = new ArrayList<List<String>>();
    for (var column : fieldMetadata.getComplexType().getChildrenList()) {
      var values = new ArrayList<String>();
      for (var feature : layer.features()) {
        if (column.getName().equals("default")) {
          var value = (String) feature.properties().get("name");
          values.add(value);
        } else {
          var value = (String) feature.properties().get("name:" + column.getName());
          values.add(value);
        }
      }
      sharedValues.add(values);
    }

    final var encodedValues =
        encodeSharedDictionary(sharedValues, PhysicalLevelTechnique.FAST_PFOR, false);
    final var decodeResult =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), fieldMetadata);
    final var decodedValues = decodeResult.getRight();

    for (var column : fieldMetadata.getComplexType().getChildrenList()) {
      var i = 0;
      for (var feature : layer.features()) {
        if (column.getName().equals("default")) {
          var value = (String) feature.properties().get("name");
          var actualValue = decodedValues.get("name").get(i++);
          if (value == null && actualValue == null) {
            continue;
          }
          Assert.equals(value, actualValue);
        } else {
          var value = (String) feature.properties().get("name:" + column.getName());
          var actualValue = decodedValues.get("name:" + column.getName()).get(i++);
          if (value == null && actualValue == null) {
            continue;
          }
          Assert.equals(value, actualValue);
        }
      }
    }
  }
}
