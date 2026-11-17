export function unpackBitpackU32Into(data, count, out) {
  if (data.length === 0) throw new Error("bitpack");
  const width = data[0];
  if (width < 1 || width > 32) throw new Error("bitpack width");
  const expectedBytes = Math.ceil((count * width) / 8);
  if (data.length - 1 !== expectedBytes) throw new Error("bitpack len");
  if (count === 0) return out;

  if (width === 8) {
    out.set(data.subarray(1, 1 + count));
    return out;
  }
  if (width <= 24) {
    // Fast 32-bit integer accumulator path (no BigInt needed when width + 7 <= 31)
    const mask = (1 << width) - 1;
    let acc = 0;
    let bits = 0;
    let p = 1;
    for (let i = 0; i < count; i++) {
      while (bits < width) {
        acc |= data[p++] << bits;
        bits += 8;
      }
      out[i] = acc & mask;
      acc >>>= width;
      bits -= width;
    }
    return out;
  }

  const mask = (1n << BigInt(width)) - 1n;
  const wBig = BigInt(width);
  let acc = 0n;
  let bits = 0;
  let p = 1;
  for (let i = 0; i < count; i++) {
    while (bits < width) {
      acc |= BigInt(data[p++]) << BigInt(bits);
      bits += 8;
    }
    out[i] = Number(acc & mask) >>> 0;
    acc >>= wBig;
    bits -= width;
  }
  return out;
}

export function unpackBitpackU64Into(data, count, u32Scratch, out) {
  unpackBitpackU32Into(data, count, u32Scratch);
  for (let i = 0; i < count; i++) out[i] = BigInt(u32Scratch[i]);
  return out;
}
