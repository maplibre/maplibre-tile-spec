package com.mlt.converter;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.mlt.TestSettings;
import com.mlt.converter.mvt.ColumnMapping;
import com.mlt.converter.mvt.MvtUtils;
import com.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.nio.file.Paths;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.Test;

public class MltConverterTest {

  @Test
  @Disabled
  // Fails currently with org.opentest4j.AssertionFailedError: expected: <STRING> but was: <name:
  // "class"
  public void createTileMetadata_Omt_ValidMetadata() throws IOException {
    var expectedPropertiesScheme =
        Map.of(
            "water", Map.of("class", MltTilesetMetadata.ScalarType.STRING),
            "waterway", Map.of("class", MltTilesetMetadata.ScalarType.STRING),
            "landuse", Map.of("class", MltTilesetMetadata.ScalarType.STRING),
            "transportation",
                Map.of(
                    "class",
                    MltTilesetMetadata.ScalarType.STRING,
                    "brunnel",
                    MltTilesetMetadata.ScalarType.STRING),
            "water_name",
                Map.of(
                    "class",
                    MltTilesetMetadata.ScalarType.STRING,
                    "intermittent",
                    MltTilesetMetadata.ScalarType.INT_64,
                    "name",
                    MltTilesetMetadata.ComplexType.STRUCT),
            "place",
                Map.of(
                    "class",
                    MltTilesetMetadata.ScalarType.STRING,
                    "iso:a2",
                    MltTilesetMetadata.ScalarType.STRING,
                    "name",
                    MltTilesetMetadata.ComplexType.STRUCT));
    var waternameNameProperties =
        List.of(
            "default", "ar", "az", "be", "bg", "br", "bs", "ca", "co", "cs", "cy", "da", "de", "el",
            "en", "eo", "es", "et", "eu", "fi", "fr", "fy", "ga", "he", "hi", "hr", "hu", "hy",
            "id", "int", "is", "it", "ja", "ka", "kk", "ko", "ku", "la", "latin", "lt", "lv", "mk",
            "ml", "mt", "nl", "no", "pl", "pt", "ro", "ru", "sk", "sl", "sr", "sr-Latn", "sv", "ta",
            "th", "tr", "uk", "zh");
    var placeNameProperties =
        Stream.concat(
                waternameNameProperties.stream(),
                List.of("am", "gd", "kn", "lb", "oc", "rm", "sq", "te", "nonlatin").stream())
            .collect(Collectors.toList());

    var tileId = String.format("%s_%s_%s", 5, 16, 21);
    var mvtFilePath = Paths.get(TestSettings.OMT_MVT_PATH, tileId + ".mvt");
    var mvTile = MvtUtils.decodeMvt(mvtFilePath);

    var mapping = new ColumnMapping("name", ":", true);
    var tileMetadata =
        MltConverter.createTilesetMetadata(mvTile, Optional.of(List.of(mapping)), true);

    assertEquals(mvTile.layers().size(), tileMetadata.getFeatureTablesCount());
    for (var i = 0; i < mvTile.layers().size(); i++) {
      var mvtLayer = mvTile.layers().get(i);
      var mltFeatureTableMetadata = tileMetadata.getFeatureTables(i);

      assertEquals(mvtLayer.name(), mltFeatureTableMetadata.getName());

      var idColumnMetadata =
          mltFeatureTableMetadata.getColumnsList().stream()
              .filter(f -> f.getName().equals("id"))
              .findFirst()
              .get();
      var expectedIdDataType =
          mvtLayer.name().equals("place")
              ? MltTilesetMetadata.ScalarType.UINT_64
              : MltTilesetMetadata.ScalarType.UINT_32;
      assertEquals(expectedIdDataType, idColumnMetadata.getScalarType().getPhysicalType());

      var geometryColumnMetadata =
          mltFeatureTableMetadata.getColumnsList().stream()
              .filter(f -> f.getName().equals("geometry"))
              .findFirst()
              .get();
      assertEquals(
          MltTilesetMetadata.ComplexType.GEOMETRY,
          geometryColumnMetadata.getComplexType().getPhysicalType());

      var expectedPropertySchema = expectedPropertiesScheme.get(mltFeatureTableMetadata.getName());
      var mltProperties = mltFeatureTableMetadata.getColumnsList();
      for (var expectedProperty : expectedPropertySchema.entrySet()) {
        var mltProperty =
            mltProperties.stream()
                .filter(p -> expectedProperty.getKey().equals(p.getName()))
                .findFirst();
        assertTrue(mltProperty.isPresent());
        var actualDataType = mltProperty.get();
        assertEquals(expectedProperty.getValue(), actualDataType);

        if (actualDataType.equals(MltTilesetMetadata.ComplexType.STRUCT)) {
          var nestedFields = mltProperty.get().getComplexType().getChildrenList();
          var expectedPropertyNames =
              mltFeatureTableMetadata.getName().equals("place")
                  ? placeNameProperties
                  : waternameNameProperties;
          assertEquals(expectedPropertyNames.size(), nestedFields.size());
          for (var child : nestedFields) {
            /* In this test all nested name:* fields are of type string */
            assertEquals(
                MltTilesetMetadata.ScalarType.STRING, child.getScalarField().getPhysicalType());
            assertTrue(expectedPropertyNames.contains(child.getName()));
          }
        }
      }
    }
  }
}
