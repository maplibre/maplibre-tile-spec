package org.maplibre.mlt.converter;

import com.google.gson.Gson;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Collection;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;

public class MLTStreamRecorderFile implements MLTStreamRecorder {
  public MLTStreamRecorderFile(@NotNull Path basePath) {
    this.basePath = basePath;
  }

  @Override
  public void setLayerName(@NotNull String layerName) {
    this.layerName = layerName;
  }

  @Override
  public boolean isActive() {
    return !layerName.isEmpty();
  }

  @Override
  public <T> void recordStream(
      String streamName, Collection<T> values, byte[] rawMetaData, byte[] rawData)
      throws IOException {
    if (layerName == null) {
      throw new IllegalStateException("Layer name must be set before recording streams");
    }
    if (rawMetaData == null && rawData == null) {
      return;
    }

    final var keyFilename = sanitizeFilename(layerName) + "_" + sanitizeFilename(streamName);

    if (rawMetaData != null && rawMetaData.length > 0) {
      final var path = basePath.resolve(keyFilename + ".meta.bin");
      Files.write(path, rawMetaData);
    }
    if (rawData != null && rawData.length > 0) {
      final var path = basePath.resolve(keyFilename + ".bin");
      Files.write(path, rawData);
    }
    if (values != null) {
      final var path = basePath.resolve(keyFilename + ".json");
      Files.writeString(path, new Gson().toJson(values));
    }
  }

  public static String sanitizeFilename(String name) {
    name = forbiddenTrailingPattern.matcher(name).replaceAll("");
    if (forbiddenFilenamePattern.matcher(name).matches()) {
      name = "_" + name;
    }
    return forbiddenCharacterPattern.matcher(name).replaceAll("_");
  }

  private String layerName;
  private final Path basePath;

  //    private record StreamKey(String layerName, String streamName) {}
  //    private record StreamValue<T>(Collection<T> values, byte[] rawMetaData, byte[] rawData) {}
  //    @Nullable
  //    Map<StreamKey, StreamValue> streamRecorder;

  // https://learn.microsoft.com/en-gb/windows/win32/fileio/naming-a-file#naming-conventions
  private static final Pattern forbiddenFilenamePattern =
      Pattern.compile("CON|PRN|AUX|NUL|(COM|LPT)[1-9¹²³]", Pattern.CASE_INSENSITIVE);
  private static final Pattern forbiddenCharacterPattern =
      Pattern.compile("[<>:\"/\\\\|?*\\x00-\\x1F~.]");
  private static final Pattern forbiddenTrailingPattern = Pattern.compile("[\\s.]$");
}
