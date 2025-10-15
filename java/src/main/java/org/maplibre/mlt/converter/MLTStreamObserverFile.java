package org.maplibre.mlt.converter;

import com.google.gson.Gson;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Collection;
import java.util.regex.Pattern;
import org.jetbrains.annotations.NotNull;

/**
 * An implementation of the MLTStreamObserver interface that writes raw stream data to files in a
 * specified directory.
 */
public class MLTStreamObserverFile implements MLTStreamObserver {
  public MLTStreamObserverFile(@NotNull Path basePath) {
    this.basePath = basePath;
  }

  @Override
  public void setLayerName(@NotNull String layerName) {
    this.layerName = layerName;
  }

  @Override
  public boolean isActive() {
    return layerName != null && !layerName.isEmpty();
  }

  @Override
  public <T> void observeStream(
      String streamName, Collection<T> values, byte[] rawMetaData, byte[] rawData)
      throws IOException {
    if (layerName == null) {
      throw new IllegalStateException("Layer name must be set before observing streams");
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

  // https://learn.microsoft.com/en-gb/windows/win32/fileio/naming-a-file#naming-conventions
  private static final Pattern forbiddenFilenamePattern =
      Pattern.compile("CON|PRN|AUX|NUL|(COM|LPT)[1-9¹²³]", Pattern.CASE_INSENSITIVE);
  private static final Pattern forbiddenCharacterPattern =
      Pattern.compile("[<>:\"/\\\\|?*\\x00-\\x1F~.]");
  private static final Pattern forbiddenTrailingPattern = Pattern.compile("[\\s.]$");
}
