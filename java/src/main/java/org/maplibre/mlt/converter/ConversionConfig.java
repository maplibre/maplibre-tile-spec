package org.maplibre.mlt.converter;

import java.util.Map;

public class ConversionConfig {
  private final boolean includeIds;
  private final boolean useAdvancedEncodingSchemes;
  private final Map<String, FeatureTableOptimizations> optimizations;
  private final boolean useMortonEncoding;

  /**
   * @param includeIds Specifies if the ids should be included into a FeatureTable.
   * @param useAdvancedEncodingSchemes Specifies if advanced encodings like FastPfor can be used.
   * @param optimizations Specifies if optimizations can be applied on a specific FeatureTable.
   */
  public ConversionConfig(
      boolean includeIds,
      boolean useAdvancedEncodingSchemes,
      Map<String, FeatureTableOptimizations> optimizations) {
    this(includeIds, useAdvancedEncodingSchemes, optimizations, true);
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useAdvancedEncodingSchemes,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean useMortonEncoding) {
    this.includeIds = includeIds;
    this.useAdvancedEncodingSchemes = useAdvancedEncodingSchemes;
    this.optimizations = optimizations;
    this.useMortonEncoding = useMortonEncoding;
  }

  public boolean includeIds() {
    return this.includeIds;
  }

  public boolean useAdvancedEncodingSchemes() {
    return this.useAdvancedEncodingSchemes;
  }

  public Map<String, FeatureTableOptimizations> optimizations() {
    return this.optimizations;
  }

  public boolean useMortonEncoding() {
    return this.useMortonEncoding;
  }
}
