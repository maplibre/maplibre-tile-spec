package org.maplibre.mlt.converter;

import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import lombok.Builder;

@Builder(builderClassName = "Builder", toBuilder = true)
public record ConversionConfig(
    Boolean includeIds,
    Boolean useFastPFOR,
    Boolean useFSST,
    TypeMismatchPolicy typeMismatchPolicy,
    Map<String, FeatureTableOptimizations> optimizations,
    Boolean preTessellatePolygons,
    Boolean useMortonEncoding,
    List<String> outlineFeatureTableNames,
    Pattern layerFilterPattern,
    Boolean layerFilterInvert,
    IntegerEncodingOption integerEncodingOption,
    IntegerEncodingOption geometryEncodingOption) {

  // We can't declare defaults on the fields of a record, so we have to put them here
  public static class Builder {
    // Allow SyntheticMltUtil to extend the builder for testing purposes
    public Builder() {}

    private Boolean includeIds = DEFAULT_INCLUDE_IDS;
    private Boolean useFastPFOR = DEFAULT_USE_FAST_PFOR;
    private Boolean useFSST = DEFAULT_USE_FSST;
    private TypeMismatchPolicy typeMismatchPolicy = DEFAULT_MISMATCH_POLICY;
    private Map<String, FeatureTableOptimizations> optimizations = Map.of();
    private Boolean preTessellatePolygons = DEFAULT_PRE_TESSELLATE_POLYGONS;
    private Boolean useMortonEncoding = DEFAULT_USE_MORTON_ENCODING;
    private List<String> outlineFeatureTableNames = List.of();
    private Pattern layerFilterPattern = null;
    private Boolean layerFilterInvert = DEFAULT_LAYER_FILTER_INVERT;
    private IntegerEncodingOption integerEncodingOption = DEFAULT_INTEGER_ENCODING;
    private IntegerEncodingOption geometryEncodingOption = DEFAULT_INTEGER_ENCODING;
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
}
