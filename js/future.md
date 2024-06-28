# Future work on the Javascript decoder

The Javascript decoder in this repo was written in June, 2024.

It is a direct port of the minimal code needed from the reference implementation in Java in order to decode an MLT tile.

The only focus so far in development has been on correctness and no effort yet has gone into performance optimization.

Due to how the MLT specification is designed and its flexibility and features, major opportunities exist to improve performance, as listed below:

1. Advanced encodings

One of the most novel aspects of the MLT specification is the highly efficient combination of column-oriented data with lightweight encodings

1. Lazy geometry decoding

TODO

1. Reducing memory allocations

TODO

1. Optimizing text decoding

TODO
