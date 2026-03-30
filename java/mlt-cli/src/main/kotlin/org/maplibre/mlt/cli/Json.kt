package org.maplibre.mlt.cli

import org.maplibre.mlt.converter.mvt.MapboxVectorTile
import org.maplibre.mlt.data.MapLibreTile
import org.maplibre.mlt.json.Json

fun MapboxVectorTile.toJson(pretty: Boolean = true): String = Json.toJson(this, pretty)

fun MapLibreTile.toJson(pretty: Boolean = true): String = Json.toJson(this, pretty)
