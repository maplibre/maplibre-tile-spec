import BitVector from "./flat/bitVector";
import Vector from "./vector";


export abstract class VariableSizeVector<T extends ArrayBuffer, K> extends Vector<T, K>{

    //TODO: switch to Uint32Array by changing the decodings
    protected constructor(name: string, protected offsetBuffer: Int32Array, dataBuffer: T, sizeOrNullabilityBuffer : number | BitVector){
        super(name, dataBuffer, sizeOrNullabilityBuffer);
    }

}
