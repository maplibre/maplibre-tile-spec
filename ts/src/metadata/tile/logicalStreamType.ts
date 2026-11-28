import type { DictionaryType } from "./dictionaryType.js";
import type { OffsetType } from "./offsetType.js";
import type { LengthType } from "./lengthType.js";

export type LogicalStreamType = {
    readonly dictionaryType?: DictionaryType;
    readonly offsetType?: OffsetType;
    readonly lengthType?: LengthType;
};
