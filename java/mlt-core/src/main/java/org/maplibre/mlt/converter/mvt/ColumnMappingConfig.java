package org.maplibre.mlt.converter.mvt;

import java.util.LinkedHashMap;
import java.util.List;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import org.jetbrains.annotations.NotNull;

/// Specifies multiple column mappings for different layers based on regex patterns
@SuppressWarnings("serial")
public class ColumnMappingConfig extends LinkedHashMap<Pattern, List<ColumnMapping>> {
  public ColumnMappingConfig() {
    super();
  }

  public static ColumnMappingConfig of(
      @NotNull Pattern pattern, @NotNull List<ColumnMapping> columnMappings) {
    final var config = new ColumnMappingConfig();
    config.put(pattern, columnMappings);
    return config;
  }

  public @Override String toString() {
    final var sb = new StringBuilder();
    for (final var entry : this.entrySet()) {
      final var pattern = entry.getKey();
      final var mappings = entry.getValue();
      final var mappingStrings =
          mappings.stream().map(Object::toString).collect(Collectors.joining(", "));
      sb.append(pattern).append(" : ").append(mappingStrings);
    }
    return sb.toString();
  }
}
