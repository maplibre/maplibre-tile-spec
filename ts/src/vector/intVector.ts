import { type IntFlatVector } from "./flat/intFlatVector";
import { type LongFlatVector } from "./flat/longFlatVector";
import { type IntConstVector } from "./constant/intConstVector";
import { type LongConstVector } from "./constant/longConstVector";
import { type IntSequenceVector } from "./sequence/intSequenceVector";
import { type LongSequenceVector } from "./sequence/longSequenceVector";
import { type DoubleFlatVector } from "./flat/doubleFlatVector";

export type IntVector =
    | IntFlatVector
    | LongFlatVector
    | DoubleFlatVector
    | IntSequenceVector
    | LongSequenceVector
    | IntConstVector
    | LongConstVector;
