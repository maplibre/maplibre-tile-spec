package org.maplibre.mlt.converter.encodings.fsst;

import java.util.*;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import nl.bartlouwers.fsst.SymbolTable;

class FsstDebug implements Fsst {
  private final Fsst java = new FsstJava();
  private final Fsst jni = new FsstJni();

  private static final AtomicLong jniTime = new AtomicLong();
  private static final AtomicLong javaTime = new AtomicLong();
  private static final AtomicLong jniSize = new AtomicLong();
  private static final AtomicLong javaSize = new AtomicLong();
  private static final AtomicInteger javaSmaller = new AtomicInteger();
  private static final AtomicInteger jniSmaller = new AtomicInteger();
  private static final AtomicBoolean printed = new AtomicBoolean(false);

  static {
    Runtime.getRuntime().addShutdownHook(new Thread(FsstDebug::printStats));
  }

  public static int weight(SymbolTable table) {
    return table.symbols().length + table.symbolLengths().length + table.compressedData().length;
  }

  @Override
  public SymbolTable encode(byte[] data) {
    final long a = System.currentTimeMillis();
    final var fromJni = FsstJni.isLoaded() ? jni.encode(data) : null;
    final long b = System.currentTimeMillis();
    final var fromJava = java.encode(data);
    final long c = System.currentTimeMillis();
    if (fromJni != null) {
      jniTime.addAndGet(b - a);
      javaTime.addAndGet(c - b);
      jniSize.addAndGet(weight(fromJni));
      javaSize.addAndGet(weight(fromJava));
      (weight(fromJava) <= weight(fromJni) ? javaSmaller : jniSmaller).incrementAndGet();
    }
    return fromJava;
  }

  public static void printStats() {
    if (!printed.getAndSet(true)) {
      System.err.println(
          "java/jni:"
              + javaTime
              + "ms/"
              + jniTime
              + "ms "
              + javaSize
              + "/"
              + jniSize
              + " "
              + (javaSize.get() * 1d / jniSize.get())
              + " jni smaller "
              + jniSmaller
              + "/"
              + (javaSmaller.get() + jniSmaller.get()));
    }
  }
}
