import type { Int32FlatVector } from "./flat/int32FlatVector.js";
import type { Int64FlatVector } from "./flat/int64FlatVector.js";
import type { Int32ConstVector } from "./constant/int32ConstVector.js";
import type { Int64ConstVector } from "./constant/int64ConstVector.js";
import type { Int32SequenceVector } from "./sequence/int32SequenceVector.js";
import type { Int64SequenceVector } from "./sequence/int64SequenceVector.js";
import type { DoubleFlatVector } from "./flat/doubleFlatVector.js";

export type IdVector =
    | Int32FlatVector
    | Int64FlatVector
    | DoubleFlatVector
    | Int32SequenceVector
    | Int64SequenceVector
    | Int32ConstVector
    | Int64ConstVector;
