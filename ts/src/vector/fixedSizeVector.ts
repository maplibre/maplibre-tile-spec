import Vector from "./vector.js";

export abstract class FixedSizeVector<T extends ArrayBufferView, K> extends Vector<T, K> {}
