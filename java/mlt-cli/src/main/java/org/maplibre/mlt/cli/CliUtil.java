package org.maplibre.mlt.cli;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

public class CliUtil {

  private CliUtil() {}

  static void createDir(Path path) {
    if (!Files.exists(path)) {
      try {
        Files.createDirectories(path);
      } catch (IOException ex) {
        System.err.println("Failed to create directory: " + path);
        ex.printStackTrace(System.err);
      }
    }
  }
}
