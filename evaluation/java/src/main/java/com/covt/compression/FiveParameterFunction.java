package com.covt.compression;

@FunctionalInterface
public interface FiveParameterFunction<T, U, V, W, X> {
    public int accept(T t, U u, V v, W w, X x);
}
