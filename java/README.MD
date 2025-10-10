# MapLibre Tile Java

[![Maven Central Version](https://img.shields.io/maven-central/v/org.maplibre/mlt)](https://central.sonatype.com/artifact/org.maplibre/mlt) ![GitHub License](https://img.shields.io/github/license/maplibre/maplibre-tile-spec) [![test](https://github.com/maplibre/maplibre-tile-spec/actions/workflows/test.yml/badge.svg)](https://github.com/maplibre/maplibre-tile-spec/actions/workflows/test.yml)

A Java library for encoding and decoding MapLibre Tile (MLT) format, providing efficient compression and fast access to vector tile data.

## Project Structure

This is a multi-project Gradle build with the following structure:

- **`mlt-core`**: Core library containing the MLT implementation
- **`mlt-cli`**: Command-line tools for encoding, decoding, and metadata generation

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

byte[] mvtData = Files.readAllBytes(Path.of("tile.mvt"));
var mvtTile = MvtUtils.decodeMvt(mvtData);

// Configure column mappings for nested properties (e.g., name:en, name:de)
var columnMappings = List.of(new ColumnMapping("name", ":", true));

var tilesetMetadata = MltConverter.createTilesetMetadata(mvtTile, columnMappings, true);

var optimization = new FeatureTableOptimizations(false, false, columnMappings);
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

var optimization = new FeatureTableOptimizations(true, true, columnMappings);
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
import org.maplibre.mlt.metadata.tileset.MltTilesetMetadata;
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

## CLI Tools

The project includes three command-line tools for working with MLT files:

### Building CLI Tools

```console
./gradlew cli
```

This creates the following JAR files in `mlt-cli/build/libs/`:
- `encode.jar` - Convert MVT to MLT format
- `decode.jar` - Decode MLT files
- `meta.jar` - Generate metadata from MVT files

### Encode Tool

Convert Mapbox Vector Tiles (MVT) to MLT format:

```console
# Convert a single tile
java -jar mlt-cli/build/libs/encode.jar --mvt input.mvt --mlt output.mlt

# Convert an entire MBTiles database
java -jar mlt-cli/build/libs/encode.jar --mbtiles input.mbtiles --mlt output.mbtiles

# Advanced conversion with optimizations
java -jar mlt-cli/build/libs/encode.jar \
  --mvt input.mvt \
  --mlt output.mlt \
  --tessellate \
  --outlines ALL \
  --advanced \
  --compress=gzip \
  --verbose
```

**Key Options:**
- `--mvt <file>`: Input MVT file
- `--mbtiles <file>`: Input MBTiles database
- `--mlt <file>`: Output MLT file
- `--tessellate`: Include tessellation data
- `--outlines <layers>`: Include outlines for specified layers
- `--advanced`: Enable advanced encoding schemes (FSST & FastPFOR)
- `--compress <type>`: Compression type (gzip, deflate, none)
- `--verbose`: Enable verbose output

### Decode Tool

Decode MLT files back to readable format:

```console
# Decode and print MLT content
java -jar mlt-cli/build/libs/decode.jar --mlt input.mlt --printmlt

# Decode with timing information
java -jar mlt-cli/build/libs/decode.jar --mlt input.mlt --timer
```

**Key Options:**
- `--mlt <file>`: Input MLT file (required)
- `--printmlt`: Print decoded MLT content
- `--timer`: Show decoding time

### Meta Tool

Generate metadata from MVT files:

```console
# Generate metadata and print to stdout
java -jar mlt-cli/build/libs/meta.jar --mvt input.mvt

# Generate metadata and save to file
java -jar mlt-cli/build/libs/meta.jar --mvt input.mvt --output metadata.json
```

**Key Options:**
- `--mvt <file>`: Input MVT file (required)
- `--output <file>`: Output metadata file (optional, defaults to stdout)

### Example Workflows

**Convert a single tile:**
```console
java -jar mlt-cli/build/libs/encode.jar \
  --mvt ../test/fixtures/simple/point-boolean.pbf \
  --mlt /tmp/point-boolean.mlt \
  --tessellate \
  --outlines ALL \
  --verbose
```

**Convert an MBTiles database:**
```console
java -jar mlt-cli/build/libs/encode.jar \
  --mbtiles input.mbtiles \
  --mlt output.mbtiles \
  --tessellate \
  --outlines ALL \
  --compress=deflate \
  --verbose
```

**Batch process multiple tiles:**
```console
for file in *.mvt; do
  java -jar mlt-cli/build/libs/encode.jar \
    --mvt "$file" \
    --mlt "${file%.mvt}.mlt" \
    --advanced
done
```

## Development

### Building the Project

```console
# Build all projects including CLI tools
./gradlew buildAll

# Build individual projects
./gradlew :mlt-core:build    # Core library only
./gradlew :mlt-cli:build     # CLI tools only
./gradlew cli                # Build CLI JARs
```

### Running Tests

```console
# Run all tests
./gradlew test

# Run tests for specific projects
./gradlew :mlt-core:test
./gradlew :mlt-cli:test
```

### Code Formatting

The project uses Spotless for code formatting:

```console
# Apply formatting
./gradlew spotlessApply

# Check formatting
./gradlew spotlessCheck
```

### Project Structure

- **`mlt-core/`**: Core library implementation
  - `src/main/java/`: Main source code
  - `src/test/java/`: Unit tests
  - `src/jmh/`: JMH benchmarks
  - `resources/`: Native compilation scripts

- **`mlt-cli/`**: Command-line tools
  - `src/main/java/`: CLI application code
  - `src/test/java/`: CLI tests

Built JARs are available in:
- Core library: `mlt-core/build/libs/`
- CLI tools: `mlt-cli/build/libs/`
