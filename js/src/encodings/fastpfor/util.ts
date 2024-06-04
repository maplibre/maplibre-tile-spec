
export function greatestMultiple(value: number, factor: number): number {
  return value - value % factor;
}

export function arraycopy(src: Uint32Array, srcPos: number, dest: Uint32Array, destPos: number, length: number): void {
  if (srcPos < 0 || destPos < 0 || length < 0 || srcPos + length > src.length || destPos + length > dest.length) {
    throw new Error('Invalid source or destination positions or length');
  } // Todo: Necessary?

  for (let i = 0; i < length; i++) {
    dest[destPos + i] = src[srcPos + i];
  }
}
