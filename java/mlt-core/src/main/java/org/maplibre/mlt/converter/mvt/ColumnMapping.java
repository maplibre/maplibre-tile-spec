package org.maplibre.mlt.converter.mvt;

import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Objects;
import java.util.regex.Pattern;

/*
 * In the converter it is currently possible to map a set of feature properties into a nested struct with a depth of one level.
 * For example a set of name:* feature properties like name:de and name:en can be mapped into a name struct.
 * This has the advantage that the dictionary (Shared Dictionary Encoding) can be shared among the nested columns.
 * */
public class ColumnMapping {
  public Pattern getPrefix() {
    return prefix;
  }

  public Pattern getDelimiter() {
    return delimiter;
  }

  public boolean getUseSharedDictionaryEncoding() {
    return useSharedDictionaryEncoding;
  }

  public boolean hasColumnNames() {
    return columnNames != null && !columnNames.isEmpty();
  }

  public Collection<String> getColumnNames() {
    return columnNames;
  }

  /// Construct a mapping based on common prefixes and a delimiter
  public ColumnMapping(Pattern prefix, Pattern delimiter, boolean useSharedDictionaryEncoding) {
    this(prefix, delimiter, null, useSharedDictionaryEncoding);
  }

  public ColumnMapping(String prefix, String delimiter, boolean useSharedDictionaryEncoding) {
    this(
        Pattern.compile(prefix, Pattern.LITERAL),
        Pattern.compile(delimiter, Pattern.LITERAL),
        null,
        useSharedDictionaryEncoding);
  }

  ///  Construct a mapping based on explicit column names
  public ColumnMapping(Collection<String> columnNames, boolean useSharedDictionaryEncoding) {
    this(null, null, columnNames, useSharedDictionaryEncoding);
  }

  private ColumnMapping(
      Pattern prefix,
      Pattern delimiter,
      Collection<String> columnNames,
      boolean useSharedDictionaryEncoding) {
    this.prefix = prefix;
    this.delimiter = delimiter;
    this.useSharedDictionaryEncoding = useSharedDictionaryEncoding;
    this.columnNames = (columnNames == null) ? Collections.emptyList() : List.copyOf(columnNames);
  }

  public boolean isMatch(String propertyName) {
    if (hasColumnNames()) {
      return columnNames.contains(propertyName);
    } else {
      // In the prefix case, the prefix alone or a prefix+delimiter+suffix match
      final var prefixMatcher = prefix.matcher(propertyName);
      if (prefixMatcher.find()) {
        if (prefixMatcher.start() == 0) {
          final var remainder = propertyName.substring(prefixMatcher.end());
          final var suffixMatcher = delimiter.matcher(remainder);
          return remainder.isEmpty() || (suffixMatcher.find() && suffixMatcher.start() == 0);
        }
      }
      return false;
    }
  }

  /// Find a matching column mapping among a collection of mappings grouped by layer name patterns
  public static ColumnMapping findMapping(
      ColumnMappingConfig patternMappings, String layerName, String propertyName) {
    return patternMappings.entrySet().stream()
        .filter(entry -> entry.getKey().matcher(layerName).matches())
        .map(entry -> findMapping(entry.getValue(), propertyName))
        .filter(Objects::nonNull)
        .findFirst()
        .orElse(null);
  }

  /// Find a matching column mapping among a collection of mappings
  public static ColumnMapping findMapping(
      Collection<ColumnMapping> columnMappings, String propertyName) {
    return columnMappings.stream().filter(m -> m.isMatch(propertyName)).findFirst().orElse(null);
  }

  @Override
  public String toString() {
    if (hasColumnNames()) {
      return String.format(
          "{columns=%s dictionary=%s}",
          String.join(", ", columnNames), useSharedDictionaryEncoding);
    } else {
      return String.format(
          "{prefix=%s separator=%s dictionary=%s}", prefix, delimiter, useSharedDictionaryEncoding);
    }
  }

  private final Pattern prefix;
  private final Pattern delimiter;
  private final boolean useSharedDictionaryEncoding;
  private final Collection<String> columnNames;
}
