package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.regex.Pattern;
import me.lemire.integercompression.IntWrapper;
import org.apache.commons.lang3.tuple.Pair;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.EnumSource;
import org.junit.jupiter.params.provider.ValueSource;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.TestUtils;
import org.maplibre.mlt.converter.ColumnMapping;
import org.maplibre.mlt.converter.ColumnMappingConfig;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.encodings.StringEncoder;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.data.Feature;
import org.maplibre.mlt.data.Layer;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import org.maplibre.mlt.util.ByteArrayUtil;
import org.maplibre.mlt.util.StreamUtil;

public class StringDecoderTest {

  public static Pair<Integer, ArrayList<byte[]>> encodeSharedDictionary(
      List<List<String>> values,
      PhysicalLevelTechnique physicalLevelTechnique,
      boolean useFsstEncoding)
      throws IOException {
    return StringEncoder.encodeSharedDictionary(values, physicalLevelTechnique, useFsstEncoding);
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

    final var isNullable = true;
    final var tileMetadata =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.scalarFieldType(MltMetadata.ScalarType.STRING, isNullable), "Test"));

    final var decodedValues =
        StringDecoder.decodeSharedDictionary(
            ByteArrayUtil.concat(encodedValues.getRight()), new IntWrapper(0), tileMetadata);

    final var v = decodedValues.getRight();
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

    final var test = createField("Test", MltMetadata.ScalarType.STRING, false);
    final var test2 = createField("Test2", MltMetadata.ScalarType.STRING, false);
    final var tileMetadata =
        new MltMetadata.Column(
            new MltMetadata.Field(MltMetadata.structFieldType(List.of(test, test2)), "Parent"));

    final var decodedValues =
        StringDecoder.decodeSharedDictionary(
            ByteArrayUtil.concat(encodedValues.getRight()), new IntWrapper(0), tileMetadata);

    final var v = decodedValues.getRight();
    Assert.equals(values1, v.get("ParentTest"));
    Assert.equals(values2, v.get("ParentTest2"));
  }

  private MltMetadata.Field createField(
      String name,
      @SuppressWarnings("SameParameterValue") MltMetadata.ScalarType type,
      boolean isNullable) {
    return new MltMetadata.Field(MltMetadata.scalarFieldType(type, isNullable), name);
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

    final var test = createField("Test", MltMetadata.ScalarType.STRING, false);
    final var test2 = createField("Test2", MltMetadata.ScalarType.STRING, false);
    final var tileMetadata =
        new MltMetadata.Column(
            new MltMetadata.Field(MltMetadata.structFieldType(List.of(test, test2)), "Parent"));

    final var decodeResults =
        StringDecoder.decodeSharedDictionary(
            ByteArrayUtil.concat(encodedValues.getRight()), new IntWrapper(0), tileMetadata);
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

    final var test = createField("Test", MltMetadata.ScalarType.STRING, false);
    final var test2 = createField("Test2", MltMetadata.ScalarType.STRING, false);
    final var tileMetadata =
        new MltMetadata.Column(
            new MltMetadata.Field(MltMetadata.structFieldType(List.of(test, test2)), "Parent"));

    final var decodeResult =
        StringDecoder.decodeSharedDictionary(
            ByteArrayUtil.concat(encodedValues.getRight()), new IntWrapper(0), tileMetadata);

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

  @ParameterizedTest
  @EnumSource(
      value = PhysicalLevelTechnique.class,
      names = {"VARINT", "FAST_PFOR"})
  public void decodeSharedDictionary_Mvt(PhysicalLevelTechnique technique) throws IOException {
    final var tileId = String.format("%s_%s_%s", 5, 16, 21);
    final var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    final var mvTile = MvtUtils.decodeMvt(mvtFilePath);
    final var values =
        mvTile
            .getLayerStream()
            .findFirst()
            .orElseThrow(() -> new IllegalArgumentException("Expected at least one layer"))
            .features()
            .stream()
            .flatMap(
                feature ->
                    feature
                        .getPropertyStream()
                        .map(p -> p.getValue(feature.getIndex()))
                        .flatMap(StreamUtil.ofType(String.class)))
            .toList();
    final var encodedValues = encodeSharedDictionary(List.of(values), technique, false);
    final var isNullable = true;
    final var tileMetadata =
        new MltMetadata.Column(
            new MltMetadata.Field(
                MltMetadata.structFieldType(
                    List.of(createField("TestChild", MltMetadata.ScalarType.STRING, isNullable))),
                "TestParent:"));
    var decodeResult =
        StringDecoder.decodeSharedDictionary(
            ByteArrayUtil.concat(encodedValues.getRight()), new IntWrapper(0), tileMetadata);
    var decodedValues = decodeResult.getRight().get("TestParent:TestChild");
    Assert.equals(values, decodedValues);
  }

  @ParameterizedTest
  @ValueSource(strings = {"water_name", "place"})
  public void decodeSharedDictionary_MvtWithNestedColumns(String tableName) throws IOException {
    final var tileId = String.format("%s_%s_%s", 5, 16, 21);
    final var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");

    // Force coverage of the case where the base mapped column does not appear in the first
    // feature, causing the metadata field to need to be modified when it is found on a later one.
    final var filter =
        new TestUtils.TileFilter() {
          private boolean matched = false;

          @Override
          public boolean test(
              Layer layer, Feature feature, String propertyKey, Object propertyValue) {
            if (!matched && layer.name().equals(tableName) && propertyKey.equals("name")) {
              matched = true;
              return false;
            }
            return true;
          }
        };
    final var mvTile = TestUtils.filterTile(MvtUtils.decodeMvt(mvtFilePath), filter);

    final var columnMapping = new ColumnMapping("name", ":", true);
    final var columnMappings =
        ColumnMappingConfig.of(Pattern.compile(".*"), List.of(columnMapping));
    final var tileMetadata = MltConverter.createTilesetMetadata(mvTile, columnMappings, true);
    final var featureTable =
        tileMetadata.featureTables.stream()
            .filter(t -> t.name().equals(tableName))
            .findFirst()
            .orElseThrow(
                () -> new IllegalArgumentException("Expected feature table  " + tableName));
    final var fieldMetadata =
        featureTable.columns().stream()
            .filter(f -> Objects.equals(f.getName(), "name"))
            .findFirst()
            .orElseThrow();

    final var layer =
        mvTile
            .getLayerStream()
            .filter(t -> t.name().equals(tableName))
            .findFirst()
            .orElseThrow(() -> new IllegalArgumentException("Expected layer  " + tableName));

    final var sharedValues =
        new ArrayList<List<String>>(fieldMetadata.field().type().complexType().children().size());
    for (var column : fieldMetadata.field().type().complexType().children()) {
      var values = new ArrayList<String>();
      for (var feature : layer.features()) {
        values.add(
            feature
                .findProperty(
                    fieldMetadata.getName() + column.name(), MltMetadata.ScalarType.STRING)
                .map(p -> p.getValue(feature.getIndex()))
                .flatMap(StreamUtil.optionalOfType(String.class))
                .orElse(null));
      }
      sharedValues.add(values);
    }

    final var encodedValues =
        encodeSharedDictionary(sharedValues, PhysicalLevelTechnique.FAST_PFOR, false);
    Assert.isTrue(encodedValues.getLeft() > 2);

    final var decodeResult =
        StringDecoder.decodeSharedDictionary(
            ByteArrayUtil.concat(encodedValues.getRight()), new IntWrapper(0), fieldMetadata);
    final var decodedValues = decodeResult.getRight();

    for (var column : fieldMetadata.field().type().complexType().children()) {
      var i = 0;
      for (var feature : layer.features()) {
        final var propertyName = fieldMetadata.getName() + column.name();
        final var expectedValue =
            feature
                .findProperty(propertyName, MltMetadata.ScalarType.STRING)
                .map(p -> p.getValue(feature.getIndex()))
                .flatMap(StreamUtil.optionalOfType(String.class))
                .orElse(null);
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
    final var columnMappings =
        ColumnMappingConfig.of(Pattern.compile(".*"), List.of(mapping1, mapping2));

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
      for (var column : table.columns()) {
        if (column.is(MltMetadata.ComplexType.STRUCT)) {
          final var complex = column.field().type().complexType();
          final var fieldKey = table.name() + ":" + column.getName();
          Assert.equals(
              expected.get(fieldKey),
              complex.children().size(),
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
    final var columnMappings =
        ColumnMappingConfig.of(Pattern.compile(".*"), List.of(columnMapping));

    final var metadata =
        MltConverter.createTilesetMetadata(mvTile, columnMappings, /*isIdPresent*/ true);

    final var expected =
        Map.of(
            "water_name:name", 5,
            "place:name", 5);
    int found = 0;
    for (var table : metadata.featureTables) {
      for (var column : table.columns()) {
        if (column.is(MltMetadata.ComplexType.STRUCT)) {
          final var complex = column.field().type().complexType();
          final var fieldKey = table.name() + ":" + column.getName();
          Assert.isTrue(expected.containsKey(fieldKey));
          Assert.equals(
              expected.get(fieldKey),
              complex.children().size(),
              "Unexpected number of children in " + fieldKey);
          found++;
        }
      }
    }
    Assert.equals(expected.size(), found);
  }
}
