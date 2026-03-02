package org.maplibre.mlt.converter.encodings.fsst;

import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;

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
    Runtime.getRuntime()
        .addShutdownHook(
            new Thread(
                () -> {
                  System.err.print(FsstDebug.printStatsOnce());
                }));
  }

  @Override
  public SymbolTable encode(byte[] data) {
    final long a = System.currentTimeMillis();
    final var fromJni = jni.encode(data);
    final long b = System.currentTimeMillis();
    final var fromJava = java.encode(data);
    final long c = System.currentTimeMillis();
    jniTime.addAndGet(b - a);
    javaTime.addAndGet(c - b);
    jniSize.addAndGet(fromJni.weight());
    javaSize.addAndGet(fromJava.weight());
    (fromJava.weight() <= fromJni.weight() ? javaSmaller : jniSmaller).incrementAndGet();
    return fromJava;
  }

  public static String printStats() {
    return new StringBuilder()
        .append("java/jni:")
        .append(javaTime)
        .append("ms/")
        .append(jniTime)
        .append("ms ")
        .append(javaSize)
        .append("/")
        .append(jniSize)
        .append(" ")
        .append(javaSize.get() * 1d / jniSize.get())
        .append(" jni smaller ")
        .append(jniSmaller)
        .append("/")
        .append(javaSmaller.get() + jniSmaller.get())
        .append("\n")
        .toString();
  }

  public static String printStatsOnce() {
    return printed.getAndSet(true) ? "" : printStats();
  }
}
