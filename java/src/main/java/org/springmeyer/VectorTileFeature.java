package org.springmeyer;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class VectorTileFeature {
  public Map<String, Object> properties;
  public int extent;
  public int type;
  public long id;

  private Pbf pbf;
  private int geometry;
  private List<String> keys;
  private List<Object> values;

  public static final String[] types = {"Unknown", "Point", "LineString", "Polygon"};

  public VectorTileFeature(Pbf pbf, int end, int extent, List<String> keys, List<Object> values) {
    this.properties = new HashMap<>();
    this.extent = extent;
    this.type = 0;

    this.pbf = pbf;
    this.geometry = -1;
    this.keys = keys;
    this.values = values;

    pbf.readFields(this::readFeature, this, end);
  }

  private void readFeature(int tag, VectorTileFeature feature, Pbf pbf) {
    if (tag == 1) {
      feature.id = pbf.readVarint();
    } else if (tag == 2) {
      feature.readTag(pbf, feature);
    } else if (tag == 3) {
      feature.type = pbf.readVarint();
    } else if (tag == 4) {
      feature.geometry = pbf.pos;
    }
  }

  private void readTag(Pbf pbf, VectorTileFeature feature) {
    int end = pbf.readVarint() + pbf.pos;

    while (pbf.pos < end) {
      String key = feature.keys.get(pbf.readVarint());
      Object value = feature.values.get(pbf.readVarint());
      feature.properties.put(key, value);
    }
  }

  public List<List<Point>> loadGeometry() {
    Pbf pbf = this.pbf;
    pbf.pos = this.geometry;

    int end = pbf.readVarint() + pbf.pos;
    int cmd = 1;
    int length = 0;
    int x = 0;
    int y = 0;
    List<List<Point>> lines = new ArrayList<>();
    List<Point> line = null;

    while (pbf.pos < end) {
      if (length <= 0) {
        int cmdLen = pbf.readVarint();
        cmd = cmdLen & 0x7;
        length = cmdLen >> 3;
      }

      length--;

      if (cmd == 1 || cmd == 2) {
        x += pbf.readSVarint();
        y += pbf.readSVarint();

        if (cmd == 1) { // moveTo
          if (line != null) lines.add(line);
          line = new ArrayList<>();
        }

        line.add(new Point(x, y));

      } else if (cmd == 7) {
        // Workaround for https://github.com/mapbox/mapnik-vector-tile/issues/90
        if (line != null) {
          line.add(line.get(0).clone()); // closePolygon
        }
      } else {
        throw new IllegalArgumentException("unknown command " + cmd);
      }
    }

    if (line != null) lines.add(line);

    return lines;
  }
}
