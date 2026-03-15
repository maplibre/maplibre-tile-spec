import type { DictionaryType } from "./dictionaryType";
import type { OffsetType } from "./offsetType";
import type { LengthType } from "./lengthType";

export type LogicalStreamType = {
    readonly dictionaryType?: DictionaryType;
    readonly offsetType?: OffsetType;
    readonly lengthType?: LengthType;
}
