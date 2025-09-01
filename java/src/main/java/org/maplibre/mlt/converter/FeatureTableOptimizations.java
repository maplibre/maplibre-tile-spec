package org.maplibre.mlt.converter;

import java.util.List;
import java.util.Optional;
import org.maplibre.mlt.converter.mvt.ColumnMapping;

/**
 * @param allowSorting Specifies if the FeatureTable can be sorted or the specified order has to be
 *     preserved.
 * @param allowIdRegeneration Specifies if the Ids can be reassigned to a feature since the have no
 *     special meaning and only has to meet the MVT spec criteria -> the value of the id SHOULD be
 *     unique among the features of the parent layer.
 * @param columnMappings Mapping of flat MVT properties to a nested MLT column to apply shared
 *     dictionary encoding among the nested columns.
 */
public record FeatureTableOptimizations(
    boolean allowSorting,
    boolean allowIdRegeneration,
    Optional<List<ColumnMapping>> columnMappings) {}
