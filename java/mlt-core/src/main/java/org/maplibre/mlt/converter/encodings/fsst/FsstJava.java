package org.maplibre.mlt.converter.encodings.fsst;

import nl.bartlouwers.fsst.SymbolTable;

class FsstJava implements Fsst {

  @Override
  public SymbolTable encode(byte[] data) {
    return SymbolTableBuilder.encode(data);
  }
}
