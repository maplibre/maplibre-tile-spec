package org.maplibre.mlt.converter.mvt;

import java.util.Arrays;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.apache.commons.lang3.NotImplementedException;
import org.apache.commons.lang3.StringUtils;

/*
 * In the converter it is currently possible to map a set of feature properties into a nested struct with a depth of one level.
 * For example a set of name:* feature properties like name:de and name:en can be mapped into a name struct.
 * This has the advantage that the dictionary (Shared Dictionary Encoding) can be shared among the nested columns.
 * */
public class ColumnMapping {
  public String getMvtPropertyPrefix() {
    return mvtPropertyPrefix;
  }

  public String getMvtDelimiter() {
    return mvtDelimiter;
  }

  public boolean getUseSharedDictionaryEncoding() {
    return useSharedDictionaryEncoding;
  }

  public Collection<String> getExplicitColumnNames() {
    return explicitColumnNames;
  }

  /// Construct a mapping based on common prefixes and a delimiter
  public ColumnMapping(
      String mvtPropertyPrefix, String mvtDelimiter, boolean useSharedDictionaryEncoding) {
    this(mvtPropertyPrefix, mvtDelimiter, useSharedDictionaryEncoding, null);
  }

  ///  Construct a mapping based on explicit column names
  public ColumnMapping(
      Collection<String> explicitColumnNames, boolean useSharedDictionaryEncoding) {
    this(null, null, useSharedDictionaryEncoding, explicitColumnNames);
    throw new NotImplementedException("Explicit column mappings are not implemented yet");
  }

  private ColumnMapping(
      String mvtPropertyPrefix,
      String mvtDelimiter,
      boolean useSharedDictionaryEncoding,
      Collection<String> explicitColumnNames) {
    if (StringUtils.isAnyBlank(mvtPropertyPrefix, mvtDelimiter)) {
      throw new IllegalArgumentException("mvtPropertyPrefix and mvtDelimiter must be non-blank");
    }
    this.mvtPropertyPrefix = mvtPropertyPrefix;
    this.mvtDelimiter = mvtDelimiter;
    this.useSharedDictionaryEncoding = useSharedDictionaryEncoding;
    this.explicitColumnNames =
        (explicitColumnNames == null) ? Collections.emptyList() : List.copyOf(explicitColumnNames);
  }

  public boolean parentMatches(String columnName) {
    // In the prefix case, the parent is the un-delimited/un-suffixed property
    if (mvtPropertyPrefix != null) {
      return columnName.equals(mvtPropertyPrefix);
    }
    // In the explicit column names case, ...
    return true;
  }

  public boolean childMatches(String propertyName) {
    // In the prefix case, the child properties consist of <prefix><delimiter><suffix>
    if (mvtPropertyPrefix != null) {
      return propertyName.startsWith(mvtPropertyPrefix + mvtDelimiter);
    }
    return false;
  }

  /// Get the suffix part of the property name.
  /// This handles cases with nested names like "name_ja_kana"
  public String getChildName(String columnName) {
    return Arrays.stream(columnName.split(mvtDelimiter))
        .skip(1)
        .collect(Collectors.joining(mvtDelimiter));
  }

  /// Find a matching column mapping among a collection of mappings grouped by layer name patterns
  /// @param parentMapping true to find parent mapping, false to find child mapping
  public static ColumnMapping findMapping(
      boolean parentMapping,
      Map<Pattern, List<ColumnMapping>> patternMappings,
      String layerName,
      String propertyName) {
    return patternMappings.entrySet().stream()
        .filter(entry -> entry.getKey().matcher(layerName).matches())
        .map(entry -> findMapping(parentMapping, entry.getValue(), propertyName))
        .filter(Objects::nonNull)
        .findFirst()
        .orElse(null);
  }

  /// Find a matching column mapping among a collection of mappings
  /// @param parentMapping true to find parent mapping, false to find child mapping
  public static ColumnMapping findMapping(
      boolean parentMapping, Collection<ColumnMapping> columnMappings, String propertyName) {
    return columnMappings.stream()
        .filter(m -> parentMapping ? m.parentMatches(propertyName) : m.childMatches(propertyName))
        .findFirst()
        .orElse(null);
  }

  @Override
  public String toString() {
    return String.format(
        "{prefix=%s separator=%s dictionary=%s}",
        mvtPropertyPrefix, mvtDelimiter, useSharedDictionaryEncoding);
  }

  private final String mvtPropertyPrefix;
  private final String mvtDelimiter;
  private final boolean useSharedDictionaryEncoding;
  private final Collection<String> explicitColumnNames;
}
