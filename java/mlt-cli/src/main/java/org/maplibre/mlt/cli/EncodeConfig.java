package org.maplibre.mlt.cli;

import jakarta.annotation.Nullable;
import java.net.URI;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;
import org.maplibre.mlt.converter.ConversionConfig;
import org.maplibre.mlt.converter.mvt.ColumnMappingConfig;

record EncodeConfig(
    @NotNull ColumnMappingConfig columnMappingConfig,
    @Nullable ConversionConfig conversionConfig,
    @Nullable URI tessellateSource,
    @Nullable Pattern sortFeaturesPattern,
    @Nullable Pattern regenIDsPattern,
    @Nullable String compressionType,
    int minZoom,
    int maxZoom,
    boolean willOutput,
    boolean willDecode,
    boolean willPrintMLT,
    boolean willPrintMVT,
    boolean compareProp,
    boolean compareGeom,
    boolean willTime,
    boolean dumpStreams,
    @NotNull TaskRunner taskRunner,
    boolean continueOnError,
    int verboseLevel) {

  public Builder asBuilder() {
    return new Builder()
        .columnMappings(this.columnMappingConfig)
        .conversionConfig(this.conversionConfig)
        .tessellateSource(this.tessellateSource)
        .sortFeaturesPattern(this.sortFeaturesPattern)
        .regenIDsPattern(this.regenIDsPattern)
        .compressionType(this.compressionType)
        .minZoom(this.minZoom)
        .maxZoom(this.maxZoom)
        .willOutput(this.willOutput)
        .willDecode(this.willDecode)
        .willPrintMLT(this.willPrintMLT)
        .willPrintMVT(this.willPrintMVT)
        .compareProp(this.compareProp)
        .compareGeom(this.compareGeom)
        .willTime(this.willTime)
        .dumpStreams(this.dumpStreams)
        .taskRunner(this.taskRunner)
        .continueOnError(this.continueOnError)
        .verboseLevel(this.verboseLevel);
  }

  public static Builder builder() {
    return new Builder();
  }

  public static final class Builder {
    private @NotNull ColumnMappingConfig columnMappingConfig = new ColumnMappingConfig();
    private @Nullable ConversionConfig conversionConfig = null;
    private @Nullable URI tessellateSource = null;
    private @Nullable Pattern sortFeaturesPattern = null;
    private @Nullable Pattern regenIDsPattern = null;
    private @Nullable String compressionType = null;
    private int minZoom = 0;
    private int maxZoom = Integer.MAX_VALUE;
    private boolean willOutput = false;
    private boolean willDecode = false;
    private boolean willPrintMLT = false;
    private boolean willPrintMVT = false;
    private boolean compareProp = false;
    private boolean compareGeom = false;
    private boolean willTime = false;
    private boolean dumpStreams = false;
    private @NotNull TaskRunner taskRunner;
    private boolean continueOnError = false;
    private int verboseLevel = 0;

    public Builder columnMappings(@NotNull ColumnMappingConfig v) {
      this.columnMappingConfig = v;
      return this;
    }

    public Builder conversionConfig(@Nullable ConversionConfig v) {
      this.conversionConfig = v;
      return this;
    }

    public Builder tessellateSource(@Nullable URI v) {
      this.tessellateSource = v;
      return this;
    }

    public Builder sortFeaturesPattern(@Nullable Pattern v) {
      this.sortFeaturesPattern = v;
      return this;
    }

    public Builder regenIDsPattern(@Nullable Pattern v) {
      this.regenIDsPattern = v;
      return this;
    }

    public Builder compressionType(@Nullable String v) {
      this.compressionType = v;
      return this;
    }

    public Builder minZoom(int v) {
      this.minZoom = v;
      return this;
    }

    public Builder maxZoom(int v) {
      this.maxZoom = v;
      return this;
    }

    public Builder willOutput(boolean v) {
      this.willOutput = v;
      return this;
    }

    public Builder willDecode(boolean v) {
      this.willDecode = v;
      return this;
    }

    public Builder willPrintMLT(boolean v) {
      this.willPrintMLT = v;
      return this;
    }

    public Builder willPrintMVT(boolean v) {
      this.willPrintMVT = v;
      return this;
    }

    public Builder compareProp(boolean v) {
      this.compareProp = v;
      return this;
    }

    public Builder compareGeom(boolean v) {
      this.compareGeom = v;
      return this;
    }

    public Builder willTime(boolean v) {
      this.willTime = v;
      return this;
    }

    public Builder dumpStreams(boolean v) {
      this.dumpStreams = v;
      return this;
    }

    public Builder taskRunner(@NotNull TaskRunner v) {
      this.taskRunner = v;
      return this;
    }

    public Builder continueOnError(boolean v) {
      this.continueOnError = v;
      return this;
    }

    public Builder verboseLevel(int v) {
      this.verboseLevel = v;
      return this;
    }

    public EncodeConfig build() {
      return new EncodeConfig(
          columnMappingConfig,
          conversionConfig,
          tessellateSource,
          sortFeaturesPattern,
          regenIDsPattern,
          compressionType,
          minZoom,
          maxZoom,
          willOutput,
          willDecode,
          willPrintMLT,
          willPrintMVT,
          compareProp,
          compareGeom,
          willTime,
          dumpStreams,
          (taskRunner != null) ? taskRunner : new SerialTaskRunner(),
          continueOnError,
          verboseLevel);
    }
  }
}
