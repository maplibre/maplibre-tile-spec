package org.maplibre.mlt.converter;

import jakarta.annotation.Nullable;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;

public class ConversionConfig {
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

  private final boolean includeIds;
  private final boolean useFastPFOR;
  private final boolean useFSST;
  private final TypeMismatchPolicy mismatchPolicy;
  private final boolean useMortonEncoding;
  private final boolean preTessellatePolygons;
  private final @NotNull Map<String, FeatureTableOptimizations> optimizations;
  private final @NotNull List<String> outlineFeatureTableNames;
  private final @Nullable Pattern layerFilterPattern;
  private final boolean layerFilterInvert;
  private final @NotNull IntegerEncodingOption integerEncodingOption;
  private final @NotNull IntegerEncodingOption geometryEncodingOption;

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
   * @param integerEncodingOption Specifies which integer encoding to use
   */
  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      TypeMismatchPolicy mismatchPolicy,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      boolean useMortonEncoding,
      List<String> outlineFeatureTableNames,
      @Nullable Pattern layerFilterPattern,
      boolean layerFilterInvert,
      @NotNull IntegerEncodingOption integerEncodingOption,
      @NotNull IntegerEncodingOption geometryEncodingOption) {
    this.includeIds = includeIds;
    this.useFastPFOR = useFastPFOR;
    this.useFSST = useFSST;
    this.mismatchPolicy = mismatchPolicy;
    this.preTessellatePolygons = preTessellatePolygons;
    this.useMortonEncoding = useMortonEncoding;
    this.optimizations = (optimizations != null) ? optimizations : new HashMap<>();
    this.outlineFeatureTableNames =
        (outlineFeatureTableNames != null) ? outlineFeatureTableNames : List.of();
    this.layerFilterPattern = layerFilterPattern;
    this.layerFilterInvert = layerFilterInvert;
    this.integerEncodingOption = integerEncodingOption;
    this.geometryEncodingOption = geometryEncodingOption;
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      TypeMismatchPolicy mismatchPolicy,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      boolean useMortonEncoding,
      List<String> outlineFeatureTableNames) {
    this(
        includeIds,
        useFastPFOR,
        useFSST,
        mismatchPolicy,
        optimizations,
        preTessellatePolygons,
        useMortonEncoding,
        outlineFeatureTableNames,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        /* integerEncodingOption= */ DEFAULT_INTEGER_ENCODING,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      boolean useMortonEncoding) {
    this(
        includeIds,
        useFastPFOR,
        useFSST,
        /* mismatchPolicy= */ DEFAULT_MISMATCH_POLICY,
        optimizations,
        preTessellatePolygons,
        useMortonEncoding,
        /* outlineFeatureTableNames= */ null,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        /* integerEncodingOption= */ DEFAULT_INTEGER_ENCODING,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      IntegerEncodingOption integerEncodingOption) {
    this(
        includeIds,
        useFastPFOR,
        useFSST,
        /* mismatchPolicy= */ DEFAULT_MISMATCH_POLICY,
        optimizations, // it was null before, now it is used
        /* preTessellatePolygons= */ DEFAULT_PRE_TESSELLATE_POLYGONS,
        /* useMortonEncoding= */ DEFAULT_USE_MORTON_ENCODING,
        /* outlineFeatureTableNames= */ null,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        integerEncodingOption,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      Map<String, FeatureTableOptimizations> optimizations) {
    this(
        includeIds,
        useFastPFOR,
        useFSST,
        /* mismatchPolicy= */ DEFAULT_MISMATCH_POLICY,
        optimizations,
        /* preTessellatePolygons= */ DEFAULT_PRE_TESSELLATE_POLYGONS,
        /* useMortonEncoding= */ DEFAULT_USE_MORTON_ENCODING,
        /* outlineFeatureTableNames= */ null,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        /* integerEncodingOption= */ DEFAULT_INTEGER_ENCODING,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public ConversionConfig(boolean includeIds, boolean useAdvancedEncodingSchemes) {
    this(
        includeIds,
        /* useFastPFOR= */ DEFAULT_USE_FAST_PFOR,
        /* useFSST= */ DEFAULT_USE_FSST,
        /* mismatchPolicy= */ DEFAULT_MISMATCH_POLICY,
        /* optimizations= */ null,
        /* preTessellatePolygons= */ DEFAULT_PRE_TESSELLATE_POLYGONS,
        /* useMortonEncoding= */ DEFAULT_USE_MORTON_ENCODING,
        /* outlineFeatureTableNames= */ null,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        /* integerEncodingOption= */ DEFAULT_INTEGER_ENCODING,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public ConversionConfig(boolean includeIds) {
    this(
        includeIds,
        /* useFastPFOR= */ DEFAULT_USE_FAST_PFOR,
        /* useFSST= */ DEFAULT_USE_FSST,
        /* mismatchPolicy= */ DEFAULT_MISMATCH_POLICY,
        /* optimizations= */ null,
        /* preTessellatePolygons= */ DEFAULT_PRE_TESSELLATE_POLYGONS,
        /* useMortonEncoding= */ DEFAULT_USE_MORTON_ENCODING,
        /* outlineFeatureTableNames= */ null,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        /* integerEncodingOption= */ DEFAULT_INTEGER_ENCODING,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public ConversionConfig() {
    this(
        /* includeIds= */ DEFAULT_INCLUDE_IDS,
        /* useFastPFOR= */ DEFAULT_USE_FAST_PFOR,
        /* useFSST= */ DEFAULT_USE_FSST,
        /* mismatchPolicy= */ DEFAULT_MISMATCH_POLICY,
        /* optimizations= */ null,
        /* preTessellatePolygons= */ DEFAULT_PRE_TESSELLATE_POLYGONS,
        /* useMortonEncoding= */ DEFAULT_USE_MORTON_ENCODING,
        /* outlineFeatureTableNames= */ null,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ DEFAULT_LAYER_FILTER_INVERT,
        /* integerEncodingOption= */ DEFAULT_INTEGER_ENCODING,
        /* geometryEncodingOption= */ DEFAULT_INTEGER_ENCODING);
  }

  public boolean getIncludeIds() {
    return this.includeIds;
  }

  public boolean getUseFastPFOR() {
    return this.useFastPFOR;
  }

  public boolean getUseFSST() {
    return this.useFSST;
  }

  public TypeMismatchPolicy getTypeMismatchPolicy() {
    return this.mismatchPolicy;
  }

  public Map<String, FeatureTableOptimizations> getOptimizations() {
    return this.optimizations;
  }

  public boolean getUseMortonEncoding() {
    return this.useMortonEncoding;
  }

  public boolean getPreTessellatePolygons() {
    return this.preTessellatePolygons;
  }

  public List<String> getOutlineFeatureTableNames() {
    return outlineFeatureTableNames;
  }

  @Nullable
  public Pattern getLayerFilterPattern() {
    return layerFilterPattern;
  }

  public boolean getLayerFilterInvert() {
    return layerFilterInvert;
  }

  public IntegerEncodingOption getIntegerEncodingOption() {
    return integerEncodingOption;
  }

  public IntegerEncodingOption getGeometryEncodingOption() {
    return geometryEncodingOption;
  }

  public Builder asBuilder() {
    return new Builder()
        .includeIds(this.includeIds)
        .useFastPFOR(this.useFastPFOR)
        .useFSST(this.useFSST)
        .mismatchPolicy(this.mismatchPolicy)
        .optimizations(this.optimizations)
        .preTessellatePolygons(this.preTessellatePolygons)
        .useMortonEncoding(this.useMortonEncoding)
        .outlineFeatureTableNames(this.outlineFeatureTableNames)
        .layerFilterPattern(this.layerFilterPattern)
        .layerFilterInvert(this.layerFilterInvert)
        .integerEncoding(this.integerEncodingOption)
        .geometryEncoding(this.geometryEncodingOption);
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
    private TypeMismatchPolicy mismatchPolicy = DEFAULT_MISMATCH_POLICY;
    private Map<String, FeatureTableOptimizations> optimizations = Map.of();
    private boolean preTessellatePolygons = DEFAULT_PRE_TESSELLATE_POLYGONS;
    private boolean useMortonEncoding = DEFAULT_USE_MORTON_ENCODING;
    private List<String> outlineFeatureTableNames = List.of();
    private Pattern layerFilterPattern = null;
    private boolean layerFilterInvert = DEFAULT_LAYER_FILTER_INVERT;
    private IntegerEncodingOption integerEncodingOption = DEFAULT_INTEGER_ENCODING;
    private @Nullable IntegerEncodingOption geometryEncodingOption = null;

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

    public Builder mismatchPolicy(TypeMismatchPolicy policy) {
      this.mismatchPolicy = policy;
      return this;
    }

    public Builder mismatchPolicy(boolean coerceMismatches, boolean elideMismatches) {
      return mismatchPolicy(
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
      var effectiveGeometryEncoding =
          geometryEncodingOption != null ? geometryEncodingOption : DEFAULT_INTEGER_ENCODING;
      return new ConversionConfig(
          includeIds,
          useFastPFOR,
          useFSST,
          mismatchPolicy,
          optimizations,
          preTessellatePolygons,
          useMortonEncoding,
          outlineFeatureTableNames,
          layerFilterPattern,
          layerFilterInvert,
          integerEncodingOption,
          effectiveGeometryEncoding);
    }
  }
}
