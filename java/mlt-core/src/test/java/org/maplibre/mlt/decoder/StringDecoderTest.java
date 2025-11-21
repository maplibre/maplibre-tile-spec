package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Objects;
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
import org.maplibre.mlt.metadata.tileset.MltMetadata;

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
  @Disabled("Dictionary decoding to a scalar column is not implemented yet")
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

    final var tileMetadata =
        new MltMetadata.Column("Test", new MltMetadata.ScalarField(MltMetadata.ScalarType.STRING));
    tileMetadata.isNullable = true;

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    var v = decodedValues.getRight();
    Assert.equals(values1, v.get(":Test"));
    Assert.equals(values2, v.get(":Test2"));
  }

  @Test
  public void decodeSharedDictionary_DictionaryEncoded() throws IOException {
    final var values1 = List.of("Test", "Test2", "Test4", "Test2", "Test");
    final var values2 = List.of("Test1", "Test2", "Test1", "Test5", "Test");
    final var values = List.of(values1, values2);
    final var encodedValues =
        encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, false);

    final var test = createField("Test", MltMetadata.ScalarType.STRING);
    final var test2 = createField("Test2", MltMetadata.ScalarType.STRING);
    final var tileMetadata = new MltMetadata.Column("Parent", createComplexColumn(test, test2));
    tileMetadata.isNullable = true;

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    var v = decodedValues.getRight();
    Assert.equals(values1, v.get("ParentTest"));
    Assert.equals(values2, v.get("ParentTest2"));
  }

  private MltMetadata.ScalarField createField(MltMetadata.ScalarType type) {
    return new MltMetadata.ScalarField(type);
  }

  private MltMetadata.Field createField(
      String name, @SuppressWarnings("SameParameterValue") MltMetadata.ScalarType type) {
    return new MltMetadata.Column(name, createField(type));
  }

  private MltMetadata.ComplexField createComplexColumn(MltMetadata.Field... fields) {
    return new MltMetadata.ComplexField(MltMetadata.ComplexType.STRUCT, Arrays.asList(fields));
  }

  @Test
  public void decodeSharedDictionary_NullValues_DictionaryEncoded() throws IOException {
    final var values1 = Arrays.asList("Test", null, "Test2", null, "Test4", "Test2", "Test");
    final var values2 =
        Arrays.asList(
            null, "Test1", "Test2", "Test1", null, null, "Test5", null, "Test", null, null);
    final var values = List.of(values1, values2);
    final var encodedValues =
        encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, false);

    final var test = createField("Test", MltMetadata.ScalarType.STRING);
    final var test2 = createField("Test2", MltMetadata.ScalarType.STRING);
    final var tileMetadata = new MltMetadata.Column("Parent", createComplexColumn(test, test2));
    tileMetadata.isNullable = true;

    final var decodeResults =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);
    final var decodedPresentValues = decodeResults.getMiddle();
    final var decodedValues = decodeResults.getRight();

    final var actualValues1 = decodedValues.get("ParentTest");
    final var p1 = decodedPresentValues.get("ParentTest");
    final var decodedV1 = new ArrayList<String>();
    var counter = 0;
    for (var i = 0; i < decodedPresentValues.size(); i++) {
      decodedV1.add(p1.get(i) ? actualValues1.get(counter++) : null);
    }
    Assert.equals(decodedV1, new ArrayList<>(Arrays.asList("Test", null)));

    final var actualValues2 = decodedValues.get("ParentTest2");
    final var p2 = decodedPresentValues.get("ParentTest2");
    final var decodedV2 = new ArrayList<String>();
    var counter2 = 0;
    for (var i = 0; i < decodedPresentValues.size(); i++) {
      decodedV2.add(p2.get(i) ? actualValues2.get(counter2++) : null);
    }
    Assert.equals(decodedV2, new ArrayList<>(Arrays.asList(null, null)));

    Assert.equals(values1, decodedValues.get("ParentTest"));
    Assert.equals(values2, decodedValues.get("ParentTest2"));
  }

  @Test
  public void decodeSharedDictionary_NullValues_FsstDictionaryEncoded() throws IOException {
    final var values1 =
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
    final var values2 =
        Arrays.asList(
            "TestTestTestTestTestTest6",
            null,
            "TestTestTestTestTestTest5",
            "TestTestTestTestTestTest8",
            "TestTestTestTestTestTes9",
            null,
            "TestTestTestTestTestTest10");
    final var values = List.of(values1, values2);
    final var encodedValues =
        encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, true);

    final var test = createField("Test", MltMetadata.ScalarType.STRING);
    final var test2 = createField("Test2", MltMetadata.ScalarType.STRING);
    final var tileMetadata = new MltMetadata.Column("Parent", createComplexColumn(test, test2));
    tileMetadata.isNullable = true;

    final var decodeResult =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), tileMetadata);

    final var decodedValues = decodeResult.getRight();
    final var decodedPresentValues = decodeResult.getMiddle();

    var actualValues1 = decodedValues.get("ParentTest");
    var p1 = decodedPresentValues.get("ParentTest");
    var decodedV1 = new ArrayList<String>();
    var counter = 0;
    for (var i = 0; i < decodedPresentValues.size(); i++) {
      decodedV1.add(p1.get(i) ? actualValues1.get(counter++) : null);
    }
    Assert.equals(decodedV1, new ArrayList<>(Arrays.asList(null, null)));

    var actualValues2 = decodedValues.get("ParentTest2");
    var p2 = decodedPresentValues.get("ParentTest2");
    var decodedV2 = new ArrayList<String>();
    var counter2 = 0;
    for (var i = 0; i < decodedPresentValues.size(); i++) {
      decodedV2.add(p2.get(i) ? actualValues2.get(counter2++) : null);
    }
    Assert.equals(decodedV2, new ArrayList<>(Arrays.asList("TestTestTestTestTestTest6", null)));

    Assert.equals(values1, decodedValues.get("ParentTest"));
    Assert.equals(values2, decodedValues.get("ParentTest2"));
  }

  /// Helper method to filter and cast stream elements
  private static <Target extends Base, Base> Function<Base, Stream<Target>> ofType(
      @SuppressWarnings("SameParameterValue") Class<Target> targetType) {
    return value ->
        targetType.isInstance(value) ? Stream.of(targetType.cast(value)) : Stream.empty();
  }

  @ParameterizedTest
  @EnumSource(
      value = PhysicalLevelTechnique.class,
      names = {"VARINT", "FAST_PFOR"})
  public void decodeSharedDictionary_Mvt(PhysicalLevelTechnique technique) throws IOException {
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
        new MltMetadata.Column(
            "TestParent:",
            createComplexColumn(createField("TestChild", MltMetadata.ScalarType.STRING)));
    tileMetadata.isNullable = true;
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

    // Force coverage of the case where the "base" mapped column (e.g., "name") doesn't appear in
    // the first feature
    mvTile.layers().getFirst().features().getFirst().properties().remove("name");

    final var columnMapping = new ColumnMapping("name", ":", true);
    final var columnMappings = Map.of(Pattern.compile(".*"), List.of(columnMapping));
    final var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);
    final var fieldMetadata =
        tileMetadata.featureTables.get(tableIndex).columns.stream()
            .filter(f -> Objects.equals(f.name, "name"))
            .findFirst()
            .orElseThrow();

    var layer = mvTile.layers().get(tableIndex);
    var sharedValues = new ArrayList<List<String>>();
    for (var column : fieldMetadata.complexType.children) {
      var values = new ArrayList<String>();
      for (var feature : layer.features()) {
        values.add((String) feature.properties().get(fieldMetadata.name + column.name));
      }
      sharedValues.add(values);
    }

    final var encodedValues =
        encodeSharedDictionary(sharedValues, PhysicalLevelTechnique.FAST_PFOR, false);
    Assert.isTrue(encodedValues.getLeft() > 2);

    final var decodeResult =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), fieldMetadata);
    final var decodedValues = decodeResult.getRight();

    for (var column : fieldMetadata.complexType.children) {
      var i = 0;
      for (var feature : layer.features()) {
        final var propertyName = fieldMetadata.name + column.name;
        final var expectedValue = (String) feature.properties().get(propertyName);
        final var field = decodedValues.get(propertyName);
        Assert.isTrue(expectedValue == null || field != null);
        final var actualValue = field.get(i++);
        if (expectedValue != null || actualValue != null) {
          Assert.equals(expectedValue, actualValue);
        }
      }
    }
  }

  /// Apply multiple column mappings with the same prefix
  @Test
  public void decodeColumnMap_Mvt_prefix_multi() throws IOException {
    final var tileId = String.format("%s_%s_%s", 5, 16, 21);
    final var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    final var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    final var mapping1 = new ColumnMapping("name", ":", true);
    final var mapping2 = new ColumnMapping("name", "_", true);
    final var columnMappings = Map.of(Pattern.compile(".*"), List.of(mapping1, mapping2));

    final var metadata =
        MltConverter.createTilesetMetadata(mvTile, columnMappings, /*isIdPresent*/ true);

    final var expected =
        Map.of(
            "water_name:name", 59,
            "water_name:name_", 3,
            "place:name", 68,
            "place:name_", 3);
    int found = 0;
    for (var table : metadata.featureTables) {
      for (var column : table.columns) {
        if (column.complexType != null
            && column.complexType.physicalType == MltMetadata.ComplexType.STRUCT) {
          final var complex = column.complexType;
          final var fieldKey = table.name + ":" + column.name;
          Assert.equals(
              expected.get(fieldKey),
              complex.children.size(),
              "Unexpected number of children in " + fieldKey);
          found++;
        }
      }
    }
    Assert.equals(expected.size(), found);
  }

  /// Apply explicit column mappings
  @Test
  public void decodeColumnMap_Mvt_explicit() throws IOException {
    final var tileId = String.format("%s_%s_%s", 5, 16, 21);
    final var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    final var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    final var columnMapping =
        new ColumnMapping(List.of("name", "name:en", "name:latin", "name_en", "name_int"), true);
    final var columnMappings = Map.of(Pattern.compile(".*"), List.of(columnMapping));

    final var metadata =
        MltConverter.createTilesetMetadata(mvTile, columnMappings, /*isIdPresent*/ true);

    final var expected =
        Map.of(
            "water_name:name", 5,
            "place:name", 5);
    int found = 0;
    for (var table : metadata.featureTables) {
      for (var column : table.columns) {
        if (column.complexType != null
            && column.complexType.physicalType == MltMetadata.ComplexType.STRUCT) {
          final var complex = column.complexType;
          final var fieldKey = table.name + ":" + column.name;
          Assert.isTrue(expected.containsKey(fieldKey));
          Assert.equals(
              expected.get(fieldKey),
              complex.children.size(),
              "Unexpected number of children in " + fieldKey);
          found++;
        }
      }
    }
    Assert.equals(expected.size(), found);
  }
}
