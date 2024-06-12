package com.mlt.metadata.stream;

public enum LogicalLevelTechnique {
  NONE,
  DELTA,
  COMPONENTWISE_DELTA,
  RLE,
  MORTON,
  /* Pseudodecimal Encoding of floats -> only for the exponent integer part an additional logical level technique is used.
   *  Both exponent and significant parts are encoded with the same physical level technique */
  PDE;
}
