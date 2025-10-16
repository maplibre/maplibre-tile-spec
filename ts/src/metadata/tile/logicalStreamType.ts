import { type DictionaryType } from './dictionaryType';
import { type OffsetType } from './offsetType';
import { type LengthType } from './lengthType';

export class LogicalStreamType {

    constructor(private readonly _dictionaryType?: DictionaryType, private readonly _offsetType?: OffsetType,
                private readonly _lengthType?: LengthType) {
    }

    get dictionaryType(): DictionaryType | undefined {
        return this._dictionaryType;
    }

    get offsetType(): OffsetType | undefined {
        return this._offsetType;
    }

    get lengthType(): LengthType | undefined {
        return this._lengthType;
    }
}
