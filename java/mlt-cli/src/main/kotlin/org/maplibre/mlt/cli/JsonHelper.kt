package org.maplibre.mlt.cli

import com.google.gson.Gson
import com.google.gson.GsonBuilder
import org.locationtech.jts.geom.Geometry
import org.locationtech.jts.io.geojson.GeoJsonWriter
import org.maplibre.mlt.converter.mvt.MapboxVectorTile
import org.maplibre.mlt.data.Feature
import org.maplibre.mlt.data.Layer
import org.maplibre.mlt.data.MapLibreTile
import java.util.SortedMap
import kotlin.math.floor

// GeoJSON does not support non-numeric floats; use Rust-style string tokens for cross-implementation consistency.
private const val F32_NAN = "f32::NAN"
private const val F32_INFINITY = "f32::INFINITY"
private const val F32_NEG_INFINITY = "f32::NEG_INFINITY"
private const val F64_NAN = "f64::NAN"
private const val F64_INFINITY = "f64::INFINITY"
private const val F64_NEG_INFINITY = "f64::NEG_INFINITY"

class JsonHelper {
    companion object {
        @JvmStatic
        fun toGeoJson(tile: MapLibreTile): String = tile.toGeoJson()
    }
}

fun MapboxVectorTile.toJson(pretty: Boolean = true): String = createGson(pretty).toJson(toJsonObjects(this))

fun MapLibreTile.toJson(pretty: Boolean = true): String = createGson(pretty).toJson(toJsonObjects(this))

fun MapLibreTile.toGeoJson(pretty: Boolean = true): String {
    val gson = createGson(pretty)
    return gson.toJson(toGeoJsonObjects(this, gson))
}

private fun createGson(pretty: Boolean): Gson {
    val builder = GsonBuilder().serializeSpecialFloatingPointValues()
    if (pretty) {
        builder.setPrettyPrinting()
    }
    return builder.create()
}

/** Recursively replace Float/Double NaN and ±Infinity with GeoJSON string tokens. */
private fun floatsAsStrings(obj: Any?): Any? =
    when (obj) {
        is Float -> {
            when {
                obj.isNaN() -> F32_NAN
                obj == Float.POSITIVE_INFINITY -> F32_INFINITY
                obj == Float.NEGATIVE_INFINITY -> F32_NEG_INFINITY
                else -> obj
            }
        }

        is Double -> {
            when {
                obj.isNaN() -> F64_NAN
                obj == Double.POSITIVE_INFINITY -> F64_INFINITY
                obj == Double.NEGATIVE_INFINITY -> F64_NEG_INFINITY
                else -> obj
            }
        }

        is Map<*, *> -> {
            obj.entries.associate { (k, v) -> k to floatsAsStrings(v) }
        }

        is List<*> -> {
            obj.map { floatsAsStrings(it) }
        }

        is Iterable<*> -> {
            obj.map { floatsAsStrings(it) }
        }

        else -> {
            obj
        }
    }

/** Recursively replace GeoJSON string tokens with Float/Double special values. */
private fun stringsAsFloats(obj: Any?): Any? =
    when (obj) {
        is String -> {
            when (obj) {
                F32_NAN -> Float.NaN
                F32_INFINITY -> Float.POSITIVE_INFINITY
                F32_NEG_INFINITY -> Float.NEGATIVE_INFINITY
                F64_NAN -> Double.NaN
                F64_INFINITY -> Double.POSITIVE_INFINITY
                F64_NEG_INFINITY -> Double.NEGATIVE_INFINITY
                else -> obj
            }
        }

        is Map<*, *> -> {
            obj.entries.associate { (k, v) -> k to stringsAsFloats(v) }
        }

        is List<*> -> {
            obj.map { stringsAsFloats(it) }
        }

        is Iterable<*> -> {
            obj.map { stringsAsFloats(it) }
        }

        else -> {
            obj
        }
    }

private fun toJsonObjects(mlTile: MapLibreTile): Map<String, Any?> =
    mutableMapOf<String, Any?>(
        "layers" to
            mlTile.layers
                .stream()
                .map { obj: Layer -> toJson(obj) }
                .toList(),
    )

private fun toJson(layer: Layer): Map<String, Any?> {
    val map = mutableMapOf<String, Any?>()
    map.put("name", layer.name)
    map.put("extent", layer.tileExtent)
    map.put(
        "features",
        layer.features
            .stream()
            .map { obj: Feature -> toJson(obj) }
            .toList(),
    )
    return map
}

private fun toJson(feature: Feature): Map<String, Any?> {
    val map = mutableMapOf<String, Any?>()
    if (feature.hasId) {
        map.put("id", feature.id)
    }
    map.put("geometry", feature.geometry.toString())
    // Print properties sorted by key and drop those with null
    // values to facilitate direct comparison with MVT output.
    map.put(
        "properties",
        feature.properties.entries
            .asSequence()
            .filter { entry -> entry.value != null }
            .associate { entry ->
                entry.key to entry.value
            },
    )
    return map
}

private fun toGeoJsonObjects(
    mlTile: MapLibreTile,
    gson: Gson,
): Map<String, Any?> {
    val fc = mutableMapOf<String, Any?>()
    fc.put("type", "FeatureCollection")
    fc.put(
        "features",
        mlTile.layers
            .stream()
            .flatMap { layer: Layer ->
                layer.features
                    .stream()
                    .map { feature: Feature ->
                        featureToGeoJson(
                            layer,
                            feature,
                            gson,
                        )
                    }
            }.toList(),
    )
    return fc
}

private fun featureToGeoJson(
    layer: Layer,
    feature: Feature,
    gson: Gson,
): Map<String, Any?> {
    val f = mutableMapOf<String, Any?>()
    f.put("type", "Feature")
    if (feature.hasId) {
        f.put("id", feature.id)
    }
    val props = getSortedNonNullProperties(feature)
    props.put("_layer", layer.name)
    props.put("_extent", layer.tileExtent)
    f.put("properties", floatsAsStrings(props))
    val geom = feature.geometry
    f.put("geometry", if (geom == null) null else geometryToGeoJson(geom, gson))
    return f
}

// Filters out null values and returns properties sorted by key.
// Duplicate keys (if any) keep the first value.
private fun getSortedNonNullProperties(feature: Feature): SortedMap<String, Any?> =
    feature.properties.entries
        .asSequence()
        .filter { entry -> entry.value != null }
        .associate { entry -> entry.key to entry.value }
        .toSortedMap()

private fun geometryToGeoJson(
    geometry: Geometry,
    gson: Gson,
): Map<String, Any?> {
    val writer = GeoJsonWriter()
    writer.setEncodeCRS(false)
    val map = gson.fromJson<MutableMap<String, Any?>>(writer.write(geometry), MutableMap::class.java)
    if (map.containsKey("coordinates")) {
        map.put("coordinates", intifyCoordinates(map.get("coordinates")))
    }
    return map.toMutableMap()
}

/** Recursively convert whole-number doubles to longs inside a coordinates structure. */
private fun intifyCoordinates(obj: Any?): Any? {
    if (obj is MutableList<*>) {
        return obj
            .stream()
            .map<Any?> { obj: Any? -> intifyCoordinates(obj) }
            .toList()
    }
    if (obj is Double && obj == floor(obj) && !obj.isInfinite() && !obj.isNaN()) {
        return obj.toLong()
    }
    return obj
}

private fun toJsonObjects(mvTile: MapboxVectorTile): Map<String, Any?> =
    mutableMapOf(
        "layers" to
            mvTile
                .layers()
                .stream()
                .map { obj: Layer -> toJson(obj) }
                .toList(),
    )
