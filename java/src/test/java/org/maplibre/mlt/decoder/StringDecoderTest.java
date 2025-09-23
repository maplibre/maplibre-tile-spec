package org.maplibre.mlt.decoder;

import java.io.IOException;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
import me.lemire.integercompression.IntWrapper;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;
import org.locationtech.jts.util.Assert;
import org.maplibre.mlt.TestSettings;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.encodings.StringEncoder;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.metadata.stream.PhysicalLevelTechnique;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;

public class StringDecoderTest {

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
    var encodedValues =
        StringEncoder.encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, true);

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
    var encodedValues =
        StringEncoder.encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, false);

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
    var encodedValues =
        StringEncoder.encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, false);

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
    var encodedValues =
        StringEncoder.encodeSharedDictionary(values, PhysicalLevelTechnique.FAST_PFOR, true);

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
  public void decodeSharedDictionary_Mvt() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var layer = mvTile.layers().get(0);
    var values = new ArrayList<String>();
    for (var feature : layer.features()) {
      var strProperties = new ArrayList<String>();
      for (var prop : feature.properties().values()) {
        if (prop instanceof String) {
          strProperties.add((String) prop);
        }
      }
      values.addAll(strProperties);
    }

    var encodedValues =
        StringEncoder.encodeSharedDictionary(
            List.of(values), PhysicalLevelTechnique.FAST_PFOR, false);

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

    var v = decodedValues.getRight().get(":Test");
    Assert.equals(values, v);
  }

  @Test
  @Disabled // Columns aren't producing complex types (anymore, yet)
  public void decodeSharedDictionary_MvtWithNestedColumns() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, Optional.of(List.of(columnMapping)), true);
    var fieldMetadata =
        tileMetadata.stream().findFirst().get().getFeatureTables(0).getColumnsList().stream()
            .filter(f -> f.getName().equals("name"))
            .findFirst()
            .get();

    var layer = mvTile.layers().get(0);
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

    var encodedValues =
        StringEncoder.encodeSharedDictionary(sharedValues, PhysicalLevelTechnique.FAST_PFOR, false);

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), fieldMetadata);

    var v = decodedValues.getRight();

    for (var column : fieldMetadata.getComplexType().getChildrenList()) {
      var i = 0;
      for (var feature : layer.features()) {
        if (column.getName().equals("default")) {
          var value = (String) feature.properties().get("name");
          var actualValue = v.get("name").get(i++);
          if (value == null && actualValue == null) {
            continue;
          }
          Assert.equals(value, actualValue);
        } else {
          var value = (String) feature.properties().get("name:" + column.getName());
          var actualValue = v.get("name:" + column.getName()).get(i++);
          if (value == null && actualValue == null) {
            continue;
          }
          Assert.equals(value, actualValue);
        }
      }
    }
  }

  @Test
  @Disabled // Columns aren't producing complex types (anymore, yet)
  public void decodeSharedDictionary_MvtWithNestedColumns2() throws IOException {
    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var columnMapping = new ColumnMapping("name", ":", true);
    var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, Optional.of(List.of(columnMapping)), true);
    var fieldMetadata =
        tileMetadata.stream()
            .skip(3)
            .findFirst()
            .get()
            .getFeatureTables(0)
            .getColumnsList()
            .stream()
            .filter(f -> f.getName().equals("name"))
            .findFirst()
            .get();

    var layer = mvTile.layers().get(3);
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

    var encodedValues =
        StringEncoder.encodeSharedDictionary(sharedValues, PhysicalLevelTechnique.FAST_PFOR, false);

    var decodedValues =
        StringDecoder.decodeSharedDictionary(
            encodedValues.getRight(), new IntWrapper(0), fieldMetadata);

    var v = decodedValues.getRight();

    for (var column : fieldMetadata.getComplexType().getChildrenList()) {
      var i = 0;
      for (var feature : layer.features()) {
        if (column.getName().equals("default")) {
          var value = (String) feature.properties().get("name");
          var actualValue = v.get("name").get(i++);
          if (value == null && actualValue == null) {
            continue;
          }
          Assert.equals(value, actualValue);
        } else {
          var value = (String) feature.properties().get("name:" + column.getName());
          var actualValue = v.get("name:" + column.getName()).get(i++);
          if (value == null && actualValue == null) {
            continue;
          }
          Assert.equals(value, actualValue);
        }
      }
    }
  }
}
