# MLT Decoder Architecture

Metadata-driven decoder that routes based on stream metadata.

## Decision Tree

```
1. PHYSICAL STREAM TYPE CHECK
   ├─ Present → Boolean decoder
   ├─ Length → Integer decoder
   ├─ Offset → Integer decoder
   └─ Data → Check logical stream type
       ├─ Dictionary → String decoder
       ├─ Offset → Integer decoder
       └─ None → Integer decoder (default)

2. WITHIN EACH DECODER
   ├─ Physical technique → How to decode raw bytes
   └─ Logical technique → What transformations to apply
```
