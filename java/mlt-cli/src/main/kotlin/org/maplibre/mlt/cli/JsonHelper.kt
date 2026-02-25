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
import kotlin.Any
import kotlin.Boolean
import kotlin.String
import kotlin.math.floor
import kotlin.streams.asSequence

object JsonHelper {
    @JvmStatic
    private fun createGson(pretty: Boolean): Gson {
        val builder = GsonBuilder().serializeSpecialFloatingPointValues()
        if (pretty) {
            builder.setPrettyPrinting()
        }
        return builder.create()
    }

    @JvmOverloads
    @JvmStatic
    fun toJson(
        mlTile: MapLibreTile,
        pretty: Boolean = true,
    ): String = createGson(pretty).toJson(toJsonObjects(mlTile))

    @JvmStatic
    fun toJsonObjects(mlTile: MapLibreTile): Map<String, Any?> =
        sortedMapOf<String, Any?>(
            "layers" to
                mlTile.layers
                    .stream()
                    .map { obj: Layer -> JsonHelper.toJson(obj) }
                    .toList(),
        )

    @JvmStatic
    private fun toJson(layer: Layer): Map<String, Any?> {
        val map = sortedMapOf<String, Any?>()
        map.put("name", layer.name)
        map.put("extent", layer.tileExtent)
        map.put(
            "features",
            layer.features
                .stream()
                .map { obj: Feature -> JsonHelper.toJson(obj) }
                .toList(),
        )
        return map
    }

    @JvmStatic
    private fun toJson(feature: Feature): Map<String, Any?> {
        val map = sortedMapOf<String, Any?>()
        if (feature.hasId) {
            map.put("id", feature.id)
        }
        map.put("geometry", feature.geometry.toString())
        // Print properties sorted by key and drop those with null
        // values to facilitate direct comparison with MVT output.
        map.put(
            "properties",
            feature.properties.entries
                .stream()
                .filter { entry -> entry.value != null }
                .asSequence()
                .associate { entry ->
                    entry.key to entry.value
                },
        )
        return map
    }

    @JvmOverloads
    @JvmStatic
    fun toGeoJson(
        mlTile: MapLibreTile,
        pretty: Boolean = true,
    ): String {
        val gson = createGson(pretty)
        return gson.toJson(toGeoJsonObjects(mlTile, gson))
    }

    @JvmStatic
    private fun toGeoJsonObjects(
        mlTile: MapLibreTile,
        gson: Gson,
    ): Map<String, Any?> {
        val fc = sortedMapOf<String, Any?>()
        fc.put("type", "FeatureCollection")
        fc.put(
            "features",
            mlTile.layers
                .stream()
                .flatMap { layer: Layer ->
                    layer.features
                        .stream()
                        .map { feature: Feature ->
                            JsonHelper.featureToGeoJson(
                                layer,
                                feature,
                                gson,
                            )
                        }
                }.toList(),
        )
        return fc
    }

    @JvmStatic
    private fun featureToGeoJson(
        layer: Layer,
        feature: Feature,
        gson: Gson,
    ): Map<String, Any?> {
        val f = sortedMapOf<String, Any?>()
        f.put("type", "Feature")
        if (feature.hasId) {
            f.put("id", feature.id)
        }
        val props = getSortedNonNullProperties(feature)
        props.put("_layer", layer.name)
        props.put("_extent", layer.tileExtent)
        f.put("properties", props)
        val geom = feature.geometry
        f.put("geometry", if (geom == null) null else geometryToGeoJson(geom, gson))
        return f
    }

    // Filters out null values and returns properties sorted by key.
    // Duplicate keys (if any) keep the first value.
    @JvmStatic
    private fun getSortedNonNullProperties(feature: Feature): SortedMap<String, Any?> =
        feature.properties.entries
            .stream()
            .filter { entry -> entry.value != null }
            .asSequence()
            .associate { entry -> entry.key to entry.value }
            .toSortedMap()

    @JvmStatic
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
        return map.toSortedMap()
    }

    /** Recursively convert whole-number doubles to longs inside a coordinates structure.  */
    @JvmStatic
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

    @JvmOverloads
    @JvmStatic
    fun toJson(
        mvTile: MapboxVectorTile,
        pretty: Boolean = true,
    ): String = createGson(pretty).toJson(toJsonObjects(mvTile))

    @JvmStatic
    private fun toJsonObjects(mvTile: MapboxVectorTile): Map<String, Any?> =
        sortedMapOf(
            "layers" to
                mvTile
                    .layers()
                    .stream()
                    .map { obj: Layer -> JsonHelper.toJson(obj) }
                    .toList(),
        )
}
