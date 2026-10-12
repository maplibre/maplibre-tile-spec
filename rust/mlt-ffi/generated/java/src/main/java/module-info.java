/**
 * Java bindings for the MapLibre Tile (MLT) native library.
 * Run with {@code --enable-native-access=org.maplibre.mlt.ffi}, or {@code ALL-UNNAMED} on the class path.
 */
module org.maplibre.mlt.ffi {
  requires static org.jspecify;

  exports org.maplibre.mlt.ffi;
}
