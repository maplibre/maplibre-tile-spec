package org.maplibre.mlt.converter;

import jakarta.annotation.Nullable;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import lombok.Builder;
import lombok.Getter;
import lombok.experimental.Accessors;
import org.jetbrains.annotations.NotNull;

@Builder(builderClassName = "ConfigBuilder", toBuilder = true)
@Accessors(fluent = true)
public class ConversionConfig {
  @NotNull @Builder.Default @Getter private final Boolean includeIds = DEFAULT_INCLUDE_IDS;
  @NotNull @Builder.Default @Getter private final Boolean useFastPFOR = DEFAULT_USE_FAST_PFOR;
  @NotNull @Builder.Default @Getter private final Boolean useFSST = DEFAULT_USE_FSST;

  @NotNull @Builder.Default @Getter
  private final TypeMismatchPolicy typeMismatchPolicy = DEFAULT_MISMATCH_POLICY;

  @NotNull @Builder.Default @Getter
  private final Map<String, FeatureTableOptimizations> optimizations = Map.of();

  @NotNull @Builder.Default @Getter
  private final Boolean preTessellatePolygons = DEFAULT_PRE_TESSELLATE_POLYGONS;

  @NotNull @Builder.Default @Getter
  private final Boolean useMortonEncoding = DEFAULT_USE_MORTON_ENCODING;

  @NotNull @Builder.Default @Getter private final List<String> outlineFeatureTableNames = List.of();
  @Nullable @Builder.Default @Getter private final Pattern layerFilterPattern = null;

  @NotNull @Builder.Default @Getter
  private final Boolean layerFilterInvert = DEFAULT_LAYER_FILTER_INVERT;

  @NotNull @Builder.Default @Getter
  private final IntegerEncodingOption integerEncodingOption = DEFAULT_INTEGER_ENCODING;

  @NotNull @Builder.Default @Getter
  private final IntegerEncodingOption geometryEncodingOption = DEFAULT_INTEGER_ENCODING;

  public static class ConfigBuilder {
    // Allow SyntheticMltUtil to extend the builder for testing purposes
    public ConfigBuilder() {}
  }

  public enum TypeMismatchPolicy {
    COERCE, // Coerce values to string on type mismatch
    ELIDE, // Skip values that don't match the first type encountered
    FAIL // Throw an error if a type mismatch is detected (default)
  }

  public enum IntegerEncodingOption {
    AUTO, // Automatically select best encoding (default)
    PLAIN, // Force plain encoding
    DELTA, // Force delta encoding
    RLE, // Force RLE encoding (only for const streams)
    DELTA_RLE // Force delta-RLE encoding
  }

  public static final boolean DEFAULT_INCLUDE_IDS = true;
  public static final boolean DEFAULT_USE_FAST_PFOR = false;
  public static final boolean DEFAULT_USE_FSST = false;
  public static final TypeMismatchPolicy DEFAULT_MISMATCH_POLICY = TypeMismatchPolicy.FAIL;
  public static final boolean DEFAULT_USE_MORTON_ENCODING = true;
  public static final boolean DEFAULT_PRE_TESSELLATE_POLYGONS = false;
  public static final boolean DEFAULT_LAYER_FILTER_INVERT = false;
  public static final IntegerEncodingOption DEFAULT_INTEGER_ENCODING = IntegerEncodingOption.AUTO;
}
