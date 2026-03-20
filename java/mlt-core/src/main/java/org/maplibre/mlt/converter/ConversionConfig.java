package org.maplibre.mlt.converter;

import jakarta.annotation.Nullable;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;

public record ConversionConfig(
    boolean includeIds,
    boolean useFastPFOR,
    boolean useFSST,
    TypeMismatchPolicy typeMismatchPolicy,
    @NotNull Map<String, FeatureTableOptimizations> optimizations,
    boolean preTessellatePolygons,
    boolean useMortonEncoding,
    @NotNull List<String> outlineFeatureTableNames,
    @Nullable Pattern layerFilterPattern,
    boolean layerFilterInvert,
    @NotNull IntegerEncodingOption integerEncodingOption,
    @NotNull IntegerEncodingOption geometryEncodingOption) {

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

  /**
   * @param includeIds Specifies if the ids should be included into a FeatureTable.
   * @param useFastPFOR Specifies if FastPfor can be used
   * @param useFSST Specifies if FSST can be used
   * @param optimizations Specifies if optimizations can be applied on a specific FeatureTable.
   * @param preTessellatePolygons Specifies if Polygons should be pre-tessellated.
   * @param useMortonEncoding Use Morton encoding
   * @param outlineFeatureTableNames A collection of names for which to include outline geometry, or
   *     'ALL' for all
   * @param layerFilterPattern A regex to filter layer names
   * @param layerFilterInvert True to invert the pattern
   * @param integerEncodingOption Specifies which integer encoding to use for property columns and
   *     other non-geometry integer streams.
   * @param geometryEncodingOption Specifies which integer encoding to use for geometry-related
   *     streams (lengths, offsets, etc.).
   */
  public ConversionConfig {
    // Normalize nullable collection arguments to defaults when null.
    optimizations = (optimizations != null) ? optimizations : Map.of();
    outlineFeatureTableNames =
        (outlineFeatureTableNames != null) ? outlineFeatureTableNames : List.of();
  }

  public Builder asBuilder() {
    return new Builder()
        .includeIds(this.includeIds())
        .useFastPFOR(this.useFastPFOR())
        .useFSST(this.useFSST())
        .typeMismatchPolicy(this.typeMismatchPolicy())
        .optimizations(this.optimizations())
        .preTessellatePolygons(this.preTessellatePolygons())
        .useMortonEncoding(this.useMortonEncoding())
        .outlineFeatureTableNames(this.outlineFeatureTableNames())
        .layerFilterPattern(this.layerFilterPattern())
        .layerFilterInvert(this.layerFilterInvert())
        .integerEncoding(this.integerEncodingOption())
        .geometryEncoding(this.geometryEncodingOption());
  }

  public static Builder builder() {
    return new Builder();
  }

  /**
   * Short builder for ConversionConfig with sensible defaults. Example:
   *
   * <pre>{@code
   * var config = ConversionConfig.builder()
   *     .includeIds(true)
   *     .useFastPFOR(true)
   *     .integerEncoding(IntegerEncodingOption.DELTA)
   *     .build();
   * }</pre>
   */
  public static class Builder {
    private boolean includeIds = DEFAULT_INCLUDE_IDS;
    private boolean useFastPFOR = DEFAULT_USE_FAST_PFOR;
    private boolean useFSST = DEFAULT_USE_FSST;
    private TypeMismatchPolicy typeMismatchPolicy = DEFAULT_MISMATCH_POLICY;
    private Map<String, FeatureTableOptimizations> optimizations = Map.of();
    private boolean preTessellatePolygons = DEFAULT_PRE_TESSELLATE_POLYGONS;
    private boolean useMortonEncoding = DEFAULT_USE_MORTON_ENCODING;
    private List<String> outlineFeatureTableNames = List.of();
    private Pattern layerFilterPattern = null;
    private boolean layerFilterInvert = DEFAULT_LAYER_FILTER_INVERT;
    private IntegerEncodingOption integerEncodingOption = DEFAULT_INTEGER_ENCODING;
    private IntegerEncodingOption geometryEncodingOption = DEFAULT_INTEGER_ENCODING;

    public Builder includeIds(boolean val) {
      this.includeIds = val;
      return this;
    }

    public Builder useFastPFOR(boolean val) {
      this.useFastPFOR = val;
      return this;
    }

    public Builder useFSST(boolean val) {
      this.useFSST = val;
      return this;
    }

    public Builder typeMismatchPolicy(TypeMismatchPolicy policy) {
      this.typeMismatchPolicy = policy;
      return this;
    }

    public Builder typeMismatchPolicy(boolean coerceMismatches, boolean elideMismatches) {
      return typeMismatchPolicy(
          coerceMismatches
              ? ConversionConfig.TypeMismatchPolicy.COERCE
              : (elideMismatches
                  ? ConversionConfig.TypeMismatchPolicy.ELIDE
                  : ConversionConfig.TypeMismatchPolicy.FAIL));
    }

    public Builder optimizations(Map<String, FeatureTableOptimizations> val) {
      this.optimizations = val;
      return this;
    }

    public Builder preTessellatePolygons(boolean val) {
      this.preTessellatePolygons = val;
      return this;
    }

    public Builder useMortonEncoding(boolean val) {
      this.useMortonEncoding = val;
      return this;
    }

    public Builder outlineFeatureTableNames(List<String> val) {
      this.outlineFeatureTableNames = val;
      return this;
    }

    public Builder layerFilterPattern(Pattern val) {
      this.layerFilterPattern = val;
      return this;
    }

    public Builder layerFilterInvert(boolean val) {
      this.layerFilterInvert = val;
      return this;
    }

    public Builder integerEncoding(IntegerEncodingOption val) {
      this.integerEncodingOption = val;
      return this;
    }

    public Builder geometryEncoding(IntegerEncodingOption val) {
      this.geometryEncodingOption = val;
      return this;
    }

    public ConversionConfig build() {
      return new ConversionConfig(
          includeIds,
          useFastPFOR,
          useFSST,
          typeMismatchPolicy,
          optimizations,
          preTessellatePolygons,
          useMortonEncoding,
          outlineFeatureTableNames,
          layerFilterPattern,
          layerFilterInvert,
          integerEncodingOption,
          geometryEncodingOption);
    }
  }
}
