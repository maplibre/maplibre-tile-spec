# MapLibre Tile Java

[![Maven Central Version](https://img.shields.io/maven-central/v/org.maplibre/mlt)](https://central.sonatype.com/artifact/org.maplibre/mlt) ![GitHub License](https://img.shields.io/github/license/maplibre/maplibre-tile-spec) [![test](https://github.com/maplibre/maplibre-tile-spec/actions/workflows/test.yml/badge.svg)](https://github.com/maplibre/maplibre-tile-spec/actions/workflows/test.yml)

A Java library for encoding and decoding MapLibre Tile (MLT) format, providing efficient compression and fast access to vector tile data.

## Project Structure

This is a multi-project Gradle build with two main components:

- **`mlt-core`** - The core library containing the MLT encoding/decoding functionality
- **`mlt-cli`** - Command-line tools for converting between MVT and MLT formats

The core library is published to Maven Central, while the CLI tools are built locally for development use.

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
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

byte[] mvtData = Files.readAllBytes(Path.of("tile.mvt"));
var mvtTile = MvtUtils.decodeMvt(mvtData);

// Configure column mappings for nested properties (e.g., name:en, name:de)
var columnMappingConfig = List.of(new ColumnMapping("name", ":", true));

var tilesetMetadata = MltConverter.createTilesetMetadata(mvtTile, columnMappingConfig, true);

var optimization = new FeatureTableOptimizations(false, false, columnMappingConfig);
var optimizations = Map.of("layer_name", optimization);
var config = new ConversionConfig(true, true, optimizations);

byte[] mltData = MltConverter.convertMvt(mvtTile, tilesetMetadata, config, null);

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

byte[] mltData = Files.readAllBytes(Path.of("tile.mlt"));

MapLibreTile tile = MltDecoder.decodeMlTile(mltData);

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

var optimization = new FeatureTableOptimizations(true, true, columnMappingConfig);
var optimizations = Map.of(
  "water", optimization,
  "roads", optimization,
  "buildings", optimization
);

var config = new ConversionConfig(true, true, optimizations);

byte[] mltData = MltConverter.convertMvt(mvtTile, tilesetMetadata, config, null);
```

### Working with Feature Properties

Access and manipulate feature properties:

```java
import org.maplibre.mlt.decoder.MltDecoder;
import org.maplibre.mlt.data.MapLibreTile;

MapLibreTile tile = MltDecoder.decodeMlTile(mltData);

for (var layer : tile.layers()) {
  for (var feature : layer.features()) {
    var properties = feature.properties();

    String name = (String) properties.get("name");
    Integer population = (Integer) properties.get("population");
    Boolean isCapital = (Boolean) properties.get("is_capital");

    // Work with nested properties (if using column mappings)
    String nameEn = (String) properties.get("name:en");
    String nameDe = (String) properties.get("name:de");

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
import org.maplibre.mlt.metadata.tileset.MltMetadata;
import java.io.IOException;
import java.util.List;
import java.util.Map;

try (Stream<Path> paths = Files.list(Paths.get("input_tiles"))) {
  paths.filter(path -> path.toString().endsWith(".mvt"))
       .forEach(mvtPath -> {
         try {
           byte[] mvtData = Files.readAllBytes(mvtPath);
           var mvtTile = MvtUtils.decodeMvt(mvtData);
           var tilesetMetadata = MltConverter.createTilesetMetadata(mvtTile, List.of(), true);
           var config = new ConversionConfig(true, true, Map.of());
           byte[] mltData = MltConverter.convertMvt(mvtTile, tilesetMetadata, config, null);

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

This will create two executable JAR files in `mlt-cli/build/libs/`:
- `encode.jar` - Convert MVT files to MLT format
- `decode.jar` - Decode MLT files

Example of converting MVT to MLT:

```console
java -jar mlt-cli/build/libs/encode.jar --mvt ../test/fixtures/simple/point-boolean.pbf --mlt /tmp/point-boolean.mlt --tessellate --outlines ALL --verbose

java -jar mlt-cli/build/libs/encode.jar --mbtiles /path/to/input.mbtiles --mlt /tmp/output.mbtiles --tessellate --outlines ALL --compress=deflate --verbose

java -jar mlt-cli/build/libs/encode.jar --pmtiles /path/to/input.pmtiles --mlt /tmp/output.pmtiles --tessellate --outlines ALL --parallel
```

Example of decoding MLT files:

```console
java -jar mlt-cli/build/libs/decode.jar -mlt /tmp/point-boolean.mlt
```

## Contributing

Run the tests:

```
just java::test
just java::test-cli
```
