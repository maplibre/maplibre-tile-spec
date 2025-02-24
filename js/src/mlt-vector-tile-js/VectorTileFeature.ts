import Point = require("@mapbox/point-geometry");

class VectorTileFeature {
  properties: { [key: string]: any } = {};
  extent: number;
  type: 0|1|2|3 = 0;
  id: number;

  _raw: any;

  constructor(feature) {
    this.properties = feature.properties;
    this.extent = feature.extent;
    this._raw = feature;
    if (feature.id !== null) {
      this.id = Number(feature.id);
    }
  }

  loadGeometry(): Point[][] {
    // TODO: optimize to avoid needing this deep copy
    const newGeometry = [];
    const oldGeometry = this._raw.loadGeometry();
    for (let i = 0; i < oldGeometry.length; i++) {
      newGeometry[i] = [];
      for (let j = 0; j < oldGeometry[i].length; j++) {
        newGeometry[i][j] = new Point(oldGeometry[i][j].x, oldGeometry[i][j].y);
      }
    }
    return newGeometry;
  }

  toGeoJSON(x: Number, y: Number, z: Number): any {
    return this._raw.toGeoJSON(x, y, z);
  }
}

export { VectorTileFeature };
