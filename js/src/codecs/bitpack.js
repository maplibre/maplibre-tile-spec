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
  if (width <= 16) {
    const outI32 = new Int32Array(out.buffer, out.byteOffset, count);
    const mask = (1 << width) - 1;
    const safeEnd = (data.length - 2) | 0;
    let acc = 0;
    let bits = 0;
    let p = 1;
    let i = 0;
    while (i < count && p <= safeEnd) {
      if (bits < width) {
        acc |= (data[p] | (data[(p + 1) | 0] << 8)) << bits;
        p = (p + 2) | 0;
        bits = (bits + 16) | 0;
      }
      outI32[i++] = acc & mask;
      acc >>>= width;
      bits = (bits - width) | 0;
    }
    for (; i < count; i++) {
      while (bits < width) {
        acc |= data[p++] << bits;
        bits += 8;
      }
      outI32[i] = acc & mask;
      acc >>>= width;
      bits -= width;
    }
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
