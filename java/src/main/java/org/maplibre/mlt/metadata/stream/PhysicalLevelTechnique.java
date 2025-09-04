package org.maplibre.mlt.metadata.stream;

public enum PhysicalLevelTechnique {
  NONE,
  /* Preferred option, tends to produce the best compression ratio and decoding performance.
   * But currently only limited to 32 bit integer. */
  FAST_PFOR,
  /* Can produce better results in combination with a heavyweight compression scheme like Gzip.
   *  Simple compression scheme where the decoder are easier to implement compared to FastPfor.*/
  VARINT,
  /* Adaptive Lossless floating-Point Compression */
  ALP
}
