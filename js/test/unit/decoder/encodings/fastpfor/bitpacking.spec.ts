import { fastunpack } from '../../../../../src/encodings/fastpfor/bitpacking'

const Bitpacking_Raw_Test1: Uint32Array = new Uint32Array([ 7, 6, 5, 4, 3, 2, 1, 7, 6, 5, 4, 3, 2, 1, 7, 6, 5, 4, 3, 2, 1, 7, 6, 5, 4, 3, 2, 1, 7, 6, 5, 4 ]);
const Bitpacking_Raw_Test2: Uint32Array = new Uint32Array([ 6, 3, 1, 15, 6, 8, 3, 4, 2, 1, 6, 8, 2, 5, 6, 7, 3, 1, 8, 0, 9, 12, 15, 3, 14, 15, 11, 1, 6, 9, 2, 1 ]);
const Bitpacking_Raw_Test3: Uint32Array = new Uint32Array([ 14, 26, 31, 24, 1, 22, 14, 27, 26, 15, 4, 33, 2, 13, 27, 6, 15, 9, 31, 29, 31, 17, 30, 23, 14, 3, 2, 10, 4, 30, 21, 27 ]);
const Bitpacking_Packed_Test1: Uint32Array = new Uint32Array([ 786774391, -1796875097, -1754096453 ]);
const Bitpacking_Packed_Test2: Uint32Array = new Uint32Array([ -798522266, 605561376, 539179402, -300164976, 143810733 ]);
const Bitpacking_Packed_Test3: Uint32Array = new Uint32Array([ -2124286322, 1138388197, 431178372, 1601565263, 550395364, 1834452008 ]);


describe("Bitpacking", () => {
  it("Bitpacking unpacking (Test 1)", async () => {
    const packed = new Uint32Array(Bitpacking_Raw_Test1.length);
    fastunpack(Bitpacking_Packed_Test1, 0, packed, 0, 3);
    expect(Bitpacking_Raw_Test1).toEqual(packed);
  });
  it("Bitpacking unpacking (Test 2)", async () => {
    const packed = new Uint32Array(Bitpacking_Raw_Test2.length);
    fastunpack(Bitpacking_Packed_Test2, 0, packed, 0, 5);
    expect(Bitpacking_Raw_Test2).toEqual(packed);
  });
  it("Bitpacking unpacking (Test 3)", async () => {
    const packed = new Uint32Array(Bitpacking_Raw_Test3.length);
    fastunpack(Bitpacking_Packed_Test3, 0, packed, 0, 6);
    expect(Bitpacking_Raw_Test3).toEqual(packed);
  });
})
