package com.mlt.converter.encodings.fsst;

import java.util.*;
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

  static {
    Runtime.getRuntime()
        .addShutdownHook(
            new Thread(
                () -> {
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
                }));
  }

  @Override
  public SymbolTable encode(byte[] data) {
    new LongSummaryStatistics();

    long a = System.currentTimeMillis();
    var fromJni = jni.encode(data);
    long b = System.currentTimeMillis();
    var fromJava = java.encode(data);
    long c = System.currentTimeMillis();
    jniTime.addAndGet(b - a);
    javaTime.addAndGet(c - b);
    jniSize.addAndGet(fromJni.weight());
    javaSize.addAndGet(fromJava.weight());
    (fromJava.weight() <= fromJni.weight() ? javaSmaller : jniSmaller).incrementAndGet();

    //    if (fromJni.weight() < fromJava.weight()) {
    //      System.err.println(
    //          "java/jni: "
    //              + (c - b)
    //              + "ms/"
    //              + (b - a)
    //              + "ms "
    //              + fromJava.weight()
    //              + "/"
    //              + fromJni.weight());
    //      System.err.println("  java: " + fromJava.symbolString());
    //      System.err.println("  jni: " + fromJni.symbolString());
    //    }
    return fromJava;
  }
}
