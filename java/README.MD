# MapLibre Tile Java

[![Maven Central Version](https://img.shields.io/maven-central/v/org.maplibre/mlt)](https://central.sonatype.com/artifact/org.maplibre/mlt) ![GitHub License](https://img.shields.io/github/license/maplibre/maplibre-tile-spec) [![test](https://github.com/maplibre/maplibre-tile-spec/actions/workflows/test.yml/badge.svg)](https://github.com/maplibre/maplibre-tile-spec/actions/workflows/test.yml)

A Java library for encoding and decoding MapLibre Tile (MLT) format, providing efficient compression and fast access to vector tile data.

## Installation

### Gradle (`build.gradle`)

```gradle
repositories {
  mavenCentral()
  maven {
    url 'https://maven.ecc.no/releases'
  }
}

dependencies {
  implementation 'org.maplibre:mlt:0.0.1'
}
```

### Gradle (`build.gradle.kts`)

```kotlin
repositories {
  mavenCentral()
  maven {
    url = uri("https://maven.ecc.no/releases")
  }
}

dependencies {
  implementation("org.maplibre:mlt:0.0.1")
}
```

### Maven

```xml
<repositories>
  <repository>
    <id>maven-central</id>
    <url>https://repo1.maven.org/maven2</url>
  </repository>
  <repository>
    <id>ecc-releases</id>
    <url>https://maven.ecc.no/releases</url>
  </repository>
</repositories>

<dependencies>
  <dependency>
    <groupId>org.maplibre</groupId>
    <artifactId>mlt</artifactId>
    <version>0.0.1</version>
  </dependency>
</dependencies>
```

## Usage Examples

### Basic MLT Encoding

Convert a Mapbox Vector Tile (MVT) to MLT format:

```java
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.converter.mvt.ColumnMapping;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

// Read MVT tile
byte[] mvtData = Files.readAllBytes(Path.of("tile.mvt"));
var mvtTile = MvtUtils.decodeMvt(mvtData);

// Configure column mappings for nested properties (e.g., name:en, name:de)
var columnMappings = List.of(new ColumnMapping("name", ":", true));

// Create tileset metadata
var tilesetMetadata = MltConverter.createTilesetMetadata(mvtTile, columnMappings, true);

// Configure conversion settings
var optimization = new FeatureTableOptimizations(false, false, columnMappings);
var optimizations = Map.of("layer_name", optimization);
var config = new ConversionConfig(true, true, optimizations);

// Convert MVT to MLT
byte[] mltData = MltConverter.convertMvt(mvtTile, tilesetMetadata, config, null);

// Save MLT tile
Files.write(Path.of("tile.mlt"), mltData);
```

### Basic MLT Decoding

The simplest way to decode an MLT tile:

```java
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.data.MapLibreTile;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

// Read MLT tile from file
byte[] mltData = Files.readAllBytes(Path.of("tile.mlt"));

// Decode the MLT tile
MapLibreTile tile = MltDecoder.decodeMlTile(mltData);

// Access layers and features
for (var layer : tile.layers()) {
  System.out.println("Layer: " + layer.name());
  System.out.println("Features: " + layer.features().size());

  for (var feature : layer.features()) {
    System.out.println("Feature ID: " + feature.id());
    System.out.println("Geometry: " + feature.geometry().getGeometryType());
    System.out.println("Properties: " + feature.properties());
  }
}
```

### Advanced Encoding with Optimizations

For better compression, you can enable advanced encoding schemes and sorting:

```java
import org.maplibre.mlt.converter.FeatureTableOptimizations;
import java.util.Map;

// Enable sorting and ID regeneration for better compression
var optimization = new FeatureTableOptimizations(true, true, columnMappings);
var optimizations = Map.of(
  "water", optimization,
  "roads", optimization,
  "buildings", optimization
);

// Use advanced encoding schemes for better compression
var config = new ConversionConfig(true, true, optimizations);

// Convert with optimizations
byte[] mltData = MltConverter.convertMvt(mvtTile, tilesetMetadata, config, null);
```

### Working with Feature Properties

Access and manipulate feature properties:

```java
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.data.MapLibreTile;

// Decode MLT tile
MapLibreTile tile = MltDecoder.decodeMlTile(mltData);

for (var layer : tile.layers()) {
  for (var feature : layer.features()) {
    // Access properties
    var properties = feature.properties();

    // Get specific property values
    String name = (String) properties.get("name");
    Integer population = (Integer) properties.get("population");
    Boolean isCapital = (Boolean) properties.get("is_capital");

    // Work with nested properties (if using column mappings)
    String nameEn = (String) properties.get("name:en");
    String nameDe = (String) properties.get("name:de");

    // Modify properties
    properties.put("processed", true);
  }
}
```

### Batch Processing

Process multiple tiles efficiently:

```java
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.stream.Stream;
import org.maplibre.mlt.converter.MltConverter;
import org.maplibre.mlt.converter.mvt.MvtUtils;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
import java.io.IOException;
import java.util.List;
import java.util.Map;

// Process all MVT files in a directory
try (Stream<Path> paths = Files.list(Paths.get("input_tiles"))) {
  paths.filter(path -> path.toString().endsWith(".mvt"))
       .forEach(mvtPath -> {
         try {
           // Convert each MVT to MLT
           byte[] mvtData = Files.readAllBytes(mvtPath);
           var mvtTile = MvtUtils.decodeMvt(mvtData);
           var tilesetMetadata = MltConverter.createTilesetMetadata(mvtTile, List.of(), true);
           var config = new ConversionConfig(true, true, Map.of());
           byte[] mltData = MltConverter.convertMvt(mvtTile, tilesetMetadata, config, null);

           // Save as MLT
           String mltPath = mvtPath.toString().replace(".mvt", ".mlt");
           Files.write(Paths.get(mltPath), mltData);

           System.out.println("Converted: " + mvtPath + " -> " + mltPath);
         } catch (IOException e) {
           System.err.println("Error processing " + mvtPath + ": " + e.getMessage());
         }
       });
}
```

## CLI Usage

To build command line tools:

```console
./gradlew cli
```

Example of converting MVT to MLT:

```console
java -jar build/libs/encode.jar --mvt ../test/fixtures/simple/point-boolean.pbf --mlt /tmp/point-boolean.mlt --tessellate --outlines ALL  --embedmetadata --verbose

java -jar build/libs/encode.jar --mbtiles /Users/timsylvester/src/ml/ml-tile-spec/test/omt-ref.mbtiles --mlt /tmp/omt-basic-mlt.mbtiles --tessellate --outlines ALL --compress=deflate --verbose
```

## Contributing

Run the tests:

```
./gradlew test
```
