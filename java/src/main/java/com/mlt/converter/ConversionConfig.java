package com.mlt.converter;

import java.util.Map;

/**
 *
 * @param includeIds Specifies if the ids should be included into a FeatureTable.
 * @param useAdvancedEncodingSchemes Specifies if advanced encodings like FastPfor can be used.
 * @param optimizations Specifies if optimizations can be applied on a specific FeatureTable.
 */
public record ConversionConfig(boolean includeIds, boolean useAdvancedEncodingSchemes,
                               Map<String, FeatureTableOptimizations> optimizations) {
}


