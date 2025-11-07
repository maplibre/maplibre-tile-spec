package org.maplibre.mlt.converter;

import jakarta.annotation.Nullable;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;

public class ConversionConfig {
  private final boolean includeIds;
  private final boolean useFastPFOR;
  private final boolean useFSST;
  private final boolean coercePropertyValues;
  private final boolean useMortonEncoding;
  private final boolean preTessellatePolygons;
  private final @NotNull Map<String, FeatureTableOptimizations> optimizations;
  private final @NotNull List<String> outlineFeatureTableNames;
  private final @Nullable Pattern layerFilterPattern;
  private final boolean layerFilterInvert;

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
   */
  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      boolean coercePropertyValues,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      boolean useMortonEncoding,
      List<String> outlineFeatureTableNames,
      @Nullable Pattern layerFilterPattern,
      boolean layerFilterInvert) {
    this.includeIds = includeIds;
    this.useFastPFOR = useFastPFOR;
    this.useFSST = useFSST;
    this.coercePropertyValues = coercePropertyValues;
    this.preTessellatePolygons = preTessellatePolygons;
    this.useMortonEncoding = useMortonEncoding;
    this.optimizations = (optimizations != null) ? optimizations : new HashMap<>();
    this.outlineFeatureTableNames =
        (outlineFeatureTableNames != null) ? outlineFeatureTableNames : List.of();
    this.layerFilterPattern = layerFilterPattern;
    this.layerFilterInvert = layerFilterInvert;
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      boolean coercePropertyValues,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons,
      boolean useMortonEncoding,
      List<String> outlineFeatureTableNames) {
    this(
        includeIds,
        useFastPFOR,
        useFSST,
        coercePropertyValues,
        optimizations,
        preTessellatePolygons,
        useMortonEncoding,
        outlineFeatureTableNames,
        /* layerFilterPattern= */ null,
        /* layerFilterInvert= */ false);
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
        /* coercePropertyValues= */ false,
        optimizations,
        preTessellatePolygons,
        useMortonEncoding,
        /* outlineFeatureTableNames= */ null);
  }

  public ConversionConfig(
      boolean includeIds,
      boolean useFastPFOR,
      boolean useFSST,
      Map<String, FeatureTableOptimizations> optimizations,
      boolean preTessellatePolygons) {
    this(
        includeIds,
        useFastPFOR,
        useFSST,
        /* coercePropertyValues= */ false,
        null,
        preTessellatePolygons,
        /* useMortonEncoding= */ true,
        /* outlineFeatureTableNames= */ null);
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
        /* coercePropertyValues= */ false,
        optimizations,
        /* preTessellatePolygons= */ false,
        /* useMortonEncoding= */ true,
        /* outlineFeatureTableNames= */ null);
  }

  public ConversionConfig(boolean includeIds, boolean useAdvancedEncodingSchemes) {
    this(
        includeIds,
        /* useFastPFOR= */ false,
        /* useFSST= */ false,
        /* coercePropertyValues= */ false,
        /* optimizations= */ null,
        /* preTessellatePolygons= */ false,
        /* useMortonEncoding= */ true,
        /* outlineFeatureTableNames= */ null);
  }

  public ConversionConfig(boolean includeIds) {
    this(
        includeIds,
        /* useFastPFOR= */ false,
        /* useFSST= */ false,
        /* coercePropertyValues= */ false,
        /* optimizations= */ null,
        /* preTessellatePolygons= */ false,
        /* useMortonEncoding= */ true,
        /* outlineFeatureTableNames= */ null);
  }

  public ConversionConfig() {
    this(
        /* includeIds= */ true,
        /* useFastPFOR= */ false,
        /* useFSST= */ false,
        /* coercePropertyValues= */ false,
        /* optimizations= */ null,
        /* preTessellatePolygons= */ false,
        /* useMortonEncoding= */ true,
        /* outlineFeatureTableNames= */ null);
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

  public boolean getCoercePropertyValues() {
    return this.coercePropertyValues;
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
}
