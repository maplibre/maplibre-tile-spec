import {IntFlatVector} from "./flat/intFlatVector";
import {LongFlatVector} from "./flat/longFlatVector";
import {IntConstVector} from "./constant/intConstVector";
import {LongConstVector} from "./constant/longConstVector";
import {IntSequenceVector} from "./sequence/intSequenceVector";
import {LongSequenceVector} from "./sequence/longSequenceVector";
import {DoubleFlatVector} from "./flat/doubleFlatVector";


export type IntVector = IntFlatVector | LongFlatVector | DoubleFlatVector |
    IntSequenceVector | LongSequenceVector | IntConstVector | LongConstVector;