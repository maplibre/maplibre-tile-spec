package org.maplibre.mlt.converter;

import java.util.List;
import java.util.Map;

// TODO: refactor -> get rid of that class and to preTessellation flag to ConversionConfig
public class RenderingOptimizedConversionConfig extends ConversionConfig {
  private final boolean preTessellatePolygons;
  private final List<String> outlineFeatureTableNames;

  /**
   * @param includeIds Specifies if the ids should be included into a FeatureTable.
   * @param useAdvancedEncodingSchemes Specifies if advanced encodings like FastPfor can be used.
   * @param optimizations Specifies if optimizations can be applied on a specific FeatureTable.
   * @param preTessellatePolygons Specifies if Polygons should be pre-tessellated.
   */
  public RenderingOptimizedConversionConfig(
      boolean includeIds,
      boolean useAdvancedEncodingSchemes,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons) {
    this(includeIds, useAdvancedEncodingSchemes, optimizations, preTessellatePolygons, List.of());
  }

  public RenderingOptimizedConversionConfig(
      boolean includeIds,
      boolean useAdvancedEncodingSchemes,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      List<String> outlineFeatureTableNames) {
    super(includeIds, useAdvancedEncodingSchemes, optimizations);
    this.preTessellatePolygons = preTessellatePolygons;
    this.outlineFeatureTableNames = outlineFeatureTableNames;
  }

  public boolean getPreTessellatePolygons() {
    return this.preTessellatePolygons;
  }

  public List<String> getOutlineFeatureTableNames() {
    return outlineFeatureTableNames;
  }
}
