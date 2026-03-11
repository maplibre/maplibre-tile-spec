import type { Int32FlatVector } from "./flat/int32FlatVector";
import type { Int64FlatVector } from "./flat/int64FlatVector";
import type { Int32ConstVector } from "./constant/int32ConstVector";
import type { Int64ConstVector } from "./constant/int64ConstVector";
import type { Int32SequenceVector } from "./sequence/int32SequenceVector";
import type { Int64SequenceVector } from "./sequence/int64SequenceVector";
import type { DoubleFlatVector } from "./flat/doubleFlatVector";

export type IdVector =
  | Int32FlatVector
  | Int64FlatVector
  | DoubleFlatVector
  | Int32SequenceVector
  | Int64SequenceVector
  | Int32ConstVector
  | Int64ConstVector;
