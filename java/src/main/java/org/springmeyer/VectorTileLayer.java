package org.springmeyer;

import java.util.ArrayList;
import java.util.List;

public class VectorTileLayer {
  // Public
  public int version = 1;
  public String name = null;
  public int extent = 4096;
  public int length = 0;

  // Private
  private Pbf pbf;
  private List<String> keys = new ArrayList<>();
  private List<Object> values = new ArrayList<>();
  private List<Integer> features = new ArrayList<>();

  public VectorTileLayer(Pbf pbf, int end) throws IllegalArgumentException {
    this.pbf = pbf;
    pbf.readFields(VectorTileLayer::readLayer, this, end);
    this.length = this.features.size();
  }

  private static void readLayer(int tag, VectorTileLayer layer, Pbf pbf)
      throws IllegalArgumentException {
    switch (tag) {
      case 15:
        layer.version = pbf.readVarint();
        break;
      case 1:
        layer.name = pbf.readString();
        break;
      case 5:
        layer.extent = pbf.readVarint();
        break;
      case 2:
        layer.features.add(pbf.pos);
        break;
      case 3:
        layer.keys.add(pbf.readString());
        break;
      case 4:
        layer.values.add(readValueMessage(pbf));
        break;
      default:
        break;
    }
  }

  private static Object readValueMessage(Pbf pbf) throws IllegalArgumentException {
    Object value = null;
    int end = pbf.readVarint() + pbf.pos;
    while (pbf.pos < end) {
      int tag = pbf.readVarint() >> 3;
      if (tag == 1) {
        value = pbf.readString();
      } else if (tag == 2) {
        value = pbf.readFloat();
      } else if (tag == 3) {
        value = pbf.readDouble();
      } else if (tag == 4) {
        value = pbf.readVarint64();
      } else if (tag == 5) {
        value = pbf.readVarint();
      } else if (tag == 6) {
        value = pbf.readSVarint();
      } else if (tag == 7) {
        value = pbf.readBoolean();
      }
    }

    return value;
  }

  public VectorTileFeature feature(int i) throws IllegalArgumentException {
    if (i < 0 || i >= this.features.size()) {
      throw new IndexOutOfBoundsException("feature index out of bounds");
    }

    this.pbf.pos = this.features.get(i).intValue();

    int end = this.pbf.readVarint() + this.pbf.pos;
    return new VectorTileFeature(this.pbf, end, this.extent, this.keys, this.values);
  }
}
