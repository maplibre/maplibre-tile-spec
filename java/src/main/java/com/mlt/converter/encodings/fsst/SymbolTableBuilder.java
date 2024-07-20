/*
MIT License

Copyright (c) 2018-2020, CWI, TU Munich, FSU Jena

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
 */

package com.mlt.converter.encodings.fsst;

import java.nio.ByteBuffer;
import java.util.*;
import java.util.stream.IntStream;
import org.jetbrains.annotations.NotNull;

/**
 * Port of the FSST-encoding algorithm from <a
 * href="https://github.com/cwida/fsst">github.com/cwida/fsst</a>.
 *
 * <p>Many of the C++ specific optimizations are excluded or replaced with the simpler python logic
 * described in <a
 * href="https://github.com/cwida/fsst/blob/master/fsstcompression.pdf">fsstcompression.pdf</a> to
 * improve readability.
 *
 * <p>Some Java-specific optimizations are added to bring performance within 50% of the C++ version
 * on real-world data.
 */
class SymbolTableBuilder {
  // TODO:
  // - improve symbol tests
  // - perf tests
  static final int MAX_SYMBOL_LENGTH = 8;
  private static final int NUM_ITERS = 6;
  public static final int DEFAULT_SAMPLE_SIZE = 30_000;
  private final int sampleSize;
  private final Symbol[] symbols = new Symbol[512];

  /** Index single-byte symbol in symbols array by their only byte. */
  private final int[] sIndex = new int[256];

  /** Index multi-byte symbols in symbols array by their first 2 bytes. */
  private final int[] sIndexByFirst2 = new int[(1 << 16) + 1];

  private int nSymbols;

  private SymbolTableBuilder(int sampleSize) {
    for (int code = 0; code < 256; code++) {
      symbols[code] = Symbol.of(code);
    }
    this.sampleSize = sampleSize;
  }

  /** Builds a symbol table with up to a 30kb sample and compresses {@code data} with it. */
  public static SymbolTable encode(byte[] data) {
    var buf = ByteBuffer.wrap(data);
    return buildSymbolTable(buf, DEFAULT_SAMPLE_SIZE).encode(buf);
  }

  public static SymbolTableBuilder buildSymbolTable(ByteBuffer data, int sampleSize) {
    // main loop: init symbol table with single-byte symbols, then iteratively combine them
    // and return the best one
    SymbolTableBuilder st = new SymbolTableBuilder(sampleSize);
    SymbolTableBuilder bestTable = st;
    long bestGain = Long.MIN_VALUE;
    Counters counters;
    Counters bestCounters = null;
    for (int i = 1; i <= NUM_ITERS; i++) {
      counters = new Counters();
      long gain = st.compressCount(counters, data, i < NUM_ITERS);
      if (gain >= bestGain) {
        bestCounters = counters;
        bestTable = st;
        bestGain = gain;
      }
      if (i < NUM_ITERS) st = st.makeTable(counters, false, sampleSize < data.capacity());
    }

    return bestTable.makeTable(bestCounters, true, sampleSize < data.capacity());
  }

  private SymbolTable encode(ByteBuffer text) {
    var encodedSymbols = new ByteArrayList();
    int[] lengths = new int[nSymbols];
    for (int i = 0; i < nSymbols; i++) {
      var symbol = this.symbols[256 + i];
      lengths[i] = symbol.length();
      encodedSymbols.add(symbol.bytes());
    }
    byte[] encodedText = encodeText(text, lengths);

    return new SymbolTable(encodedSymbols.toArray(), lengths, encodedText);
  }

  private byte[] encodeText(ByteBuffer text, int[] lens) {
    int cap = text.capacity();
    var encoded = new ByteArrayList(cap);
    for (int i = 0; i < cap; ) {
      int pos = findLongestSymbol(text, i);
      if (pos <= 255) {
        encoded.add(255, text.get(i++));
      } else {
        pos -= 256;
        encoded.add(pos);
        i += lens[pos];
      }
    }
    return encoded.toArray();
  }

  private static boolean isEscapeCode(int code) {
    return code < 256;
  }

  private static void addOrInc(Map<Symbol, Long> cands, Symbol s, long count, int min) {
    if (count >= min) {
      long gain = count * s.length();
      cands.merge(s, gain, Long::sum);
    }
  }

  private void add(Symbol symbol) {
    symbols[256 + (nSymbols++)] = symbol;
  }

  private int findLongestSymbol(ByteBuffer text, int offset) {
    // first look for a multi-byte symbol starting with the next 2 bytes
    if (text.capacity() - offset >= 2) {
      int a = text.getShort(offset) & 0xFFFF;
      int start = sIndexByFirst2[a];
      if (start > 0) {
        int end = sIndexByFirst2[a + 1];
        for (int code = start; code < end; code++) {
          if (symbols[code].match(text, offset, 2)) {
            return code;
          }
        }
      }
    }

    // if not found, then look for a single-byte symbol
    var letter = text.get(offset) & 0xFF;
    int start = sIndex[letter];

    if (start >= 256) {
      return start;
    }

    // otherwise just return the "escape code" for this symbol since it's not in the table
    return letter;
  }

  record Range(int start, int end) {}

  private List<Range> ranges(int size) {
    if (size < sampleSize) {
      return List.of(new Range(0, size));
    } else {
      int chunkSize = 1000;
      int samples = sampleSize / chunkSize;
      int offset = size / (samples);
      return IntStream.range(0, samples)
          .mapToObj(i -> new Range(i * offset, Math.min(size, i * offset + chunkSize)))
          .toList();
    }
  }

  private long compressCount(Counters counters, ByteBuffer text, boolean secondPass) {
    if (text.capacity() == 0) return 0;
    long gain = 0;

    for (var range : ranges(text.capacity())) {
      int start = range.start;
      int end = range.end;
      int code2;
      int code1 = findLongestSymbol(text, start);
      Symbol symbol = symbols[code1];
      int cur = start + symbol.length();
      start = cur;
      gain += symbol.length() - (1 + (isEscapeCode(code1) ? 1 : 0));
      while (cur < end) {
        // count single symbol (i.e. an option is not extending it)
        counters.count1Inc(code1);
        // as an alternative, consider just using the next byte..
        if (symbol.length() > 1) { // .. but do not count single byte symbols doubly
          counters.count1Inc(text.get(start) & 0xFF);
        }

        // now match a new symbol
        start = cur;
        code2 = findLongestSymbol(text, cur);
        Symbol symbol2 = symbols[code2];
        cur += symbol2.length();
        gain += symbol2.length() - (1 + (isEscapeCode(code2) ? 1 : 0));
        if (secondPass) { // no need to count pairs in final round
          // consider the symbol that is the concatenation of the two last symbols
          counters.count2Inc(code1, code2);
          // as an alternative, consider just extending with the next byte..
          if (symbol2.length() > 1) { // ..but do not count single byte extensions doubly
            counters.count2Inc(code1, text.get(start) & 0xFF);
          }
        }
        code1 = code2;
        symbol = symbols[code1];
      }
    }
    return gain;
  }

  private SymbolTableBuilder makeTable(Counters counters, boolean lastPass, boolean sampled) {
    int minCount = 5;
    // hashmap of c (needed because we can generate duplicate candidates)
    Map<Symbol, Long> cands = new HashMap<>();
    int max = 256 + nSymbols;
    for (int pos1 = 0; pos1 < max; pos1++) {
      int cnt1 = counters.count1GetNext(pos1);
      if (cnt1 <= 0) continue;
      Symbol s1 = symbols[pos1];
      if (!lastPass || sampled) {
        // heuristic: promoting single-byte symbols (*8) helps reduce exception rates and increases
        // [de]compression speed
        addOrInc(cands, s1, (s1.length() == 1 ? 8L : 1L) * cnt1, lastPass ? 1 : minCount);
      } else {
        addOrInc(cands, s1, (long) s1.length() * cnt1, 1);
      }

      // don't need pair-wise counts for last pass to just encode the data
      if (lastPass || s1.length() == MAX_SYMBOL_LENGTH) continue;
      for (int pos2 = 0; pos2 < max; pos2++) {
        int cnt2 = counters.count2GetNext(pos1, pos2);
        if (cnt2 < minCount) continue;
        addOrInc(cands, Symbol.concat(s1, symbols[pos2]), cnt2, minCount);
      }
    }

    // TODO inline QSymbol here?
    PriorityQueue<QSymbol> pq = new PriorityQueue<>();
    for (var entry : cands.entrySet()) {
      pq.add(new QSymbol(entry.getValue(), entry.getKey()));
    }
    SymbolTableBuilder st = new SymbolTableBuilder(sampleSize);
    while (st.nSymbols < 255 && !pq.isEmpty()) {
      var symb = pq.remove();
      if (!lastPass || sampled) {
        st.add(symb.symbol);
      } else {
        // adding a symbol costs length + 1, so don't add if it costs more than it saves
        long costs = symb.symbol.length() + 1L;
        long saves = symb.symbol.length() == 1 ? symb.gain / 8 : symb.gain;
        if (saves > costs) {
          st.add(symb.symbol);
        }
      }
    }

    return st.finish();
  }

  public SymbolTableBuilder finish() {
    Symbol[] tmp = Arrays.copyOfRange(symbols, 256, 256 + nSymbols);
    Arrays.sort(tmp); // sorts prefix symbols after the longer symbols
    for (int i = nSymbols - 1; i >= 0; i--) {
      int letter = tmp[i].first();
      // index multi-byte by their first 2 bytes
      if (tmp[i].length() >= 2) {
        var bytes = tmp[i].bytes();
        int val = ((bytes[0] & 0xFF) << 8) | (bytes[1] & 0xFF);
        sIndexByFirst2[val] = 256 + i;
        // there might be symbols with this prefix, so store end of the range by setting val+1
        if (sIndexByFirst2[val + 1] == 0) {
          sIndexByFirst2[val + 1] = 256 + i + 1;
        }
      } else {
        // index single-byte symbols by their only byte
        sIndex[letter] = 256 + i;
      }
      symbols[256 + i] = tmp[i];
    }
    return this;
  }

  private record QSymbol(long gain, Symbol symbol) implements Comparable<QSymbol> {
    @Override
    public int compareTo(@NotNull SymbolTableBuilder.QSymbol o) {
      return Long.compare(o.gain, gain);
    }
  }

  static class Counters {
    private final int[] count1 = new int[512];
    private final int[] count2 = new int[512 * 512];

    public void count1Inc(int pos1) {
      count1[pos1]++;
    }

    public void count2Inc(int pos1, int pos2) {
      count2[(pos1 << 9) | pos2]++;
    }

    public int count1GetNext(int pos1) {
      return count1[pos1];
    }

    public int count2GetNext(int pos1, int pos2) {
      return count2[(pos1 << 9) | pos2];
    }
  }
}
