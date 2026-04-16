"""QGIS plugin entry point for MLT (MapLibre Tile) file support."""


def classFactory(iface):
    from .plugin import MltPlugin

    return MltPlugin(iface)
