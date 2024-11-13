import { DictionaryType } from './DictionaryType';
import { OffsetType } from './OffsetType';
import { LengthType } from './LengthType';

export class LogicalStreamType {
    private dictionary_type: DictionaryType | undefined;
    private offset_type: OffsetType | undefined;
    private length_type: LengthType | undefined;

    constructor(dictionary_type?: DictionaryType, offset_type?: OffsetType, length_type?: LengthType) {
        if (dictionary_type) {
            this.dictionary_type = dictionary_type;
        } else if (offset_type) {
            this.offset_type = offset_type;
        } else if (length_type) {
            this.length_type = length_type;
        }
    }

    public dictionaryType(): DictionaryType | undefined {
        return this.dictionary_type;
    }

    public offsetType(): OffsetType | undefined {
        return this.offset_type;
    }

    public lengthType(): LengthType | undefined {
        return this.length_type;
    }
}