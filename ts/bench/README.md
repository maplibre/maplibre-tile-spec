# Benchmarks

Benchmarks are intentionally **not** part of `npm test` / CI.

## FastPFOR vs Varint decoding

From `ts/`:

```bash
npm run bench:fastpfor
```

Add `--big` to include a larger dataset (multi-page):

```bash
npm run bench:fastpfor -- --big
```

Alternatively, use an environment variable:

```bash
BENCH_BIG=1 npm run bench:fastpfor
```
