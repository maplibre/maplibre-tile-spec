"""QGIS Plugin class that registers the MLT file handler."""

from pathlib import Path

from qgis.PyQt.QtGui import QIcon
from qgis.PyQt.QtWidgets import QAction, QFileDialog
from qgis.core import Qgis, QgsMessageLog

from .loader import load_mlt_file, load_mlt_files_merged
from .tile_coords import (
    MultipleTileCoordDialog,
    TileCoordDialog,
    parse_zxy_from_path,
)

PLUGIN_NAME = "MLT Provider"


class MltPlugin:
    def __init__(self, iface):
        self.iface = iface
        self.action = None

    def initGui(self):
        icon = QIcon()
        self.action = QAction(icon, "Open MLT File(s)\u2026", self.iface.mainWindow())
        self.action.triggered.connect(self.open_file_dialog)
        self.iface.addToolBarIcon(self.action)
        self.iface.addPluginToVectorMenu(PLUGIN_NAME, self.action)

    def unload(self):
        if self.action:
            self.iface.removePluginVectorMenu(PLUGIN_NAME, self.action)
            self.iface.removeToolBarIcon(self.action)

    def open_file_dialog(self):
        paths, _ = QFileDialog.getOpenFileNames(
            self.iface.mainWindow(),
            "Open MapLibre Tile(s)",
            "",
            "MLT Files (*.mlt);;All Files (*)",
        )
        if not paths:
            return

        if len(paths) == 1:
            self._load_single(paths[0])
        else:
            self._load_multiple(paths)

    def _load_single(self, path: str):
        guess = parse_zxy_from_path(path)
        dlg = TileCoordDialog(self.iface.mainWindow(), initial=guess)
        zxy = None
        tms = True
        if dlg.exec_():
            zxy = dlg.zxy()
            tms = dlg.tms
        elif not dlg.skipped:
            return

        try:
            layers = load_mlt_file(path, zxy=zxy, tms=tms)
            self._report_result(layers, [path], zxy is not None)
        except Exception as exc:
            self._report_error(exc)

    def _load_multiple(self, paths: list):
        files_with_coords = [
            (p, parse_zxy_from_path(p)) for p in paths
        ]
        dlg = MultipleTileCoordDialog(
            self.iface.mainWindow(), files_with_coords=files_with_coords
        )

        use_coords = False
        tms = True
        merge = True
        file_coords = None

        if dlg.exec_():
            use_coords = True
            tms = dlg.tms
            merge = dlg.merge
            file_coords = dlg.file_coords()
        elif not dlg.skipped:
            return
        else:
            merge = True

        try:
            if merge:
                layers = load_mlt_files_merged(
                    paths,
                    file_coords=file_coords if use_coords else None,
                    tms=tms,
                )
            else:
                layers = []
                for p in paths:
                    zxy = file_coords.get(p) if file_coords else None
                    layers.extend(load_mlt_file(p, zxy=zxy, tms=tms))

            self._report_result(layers, paths, use_coords)
        except Exception as exc:
            self._report_error(exc)

    def _report_result(self, layers, paths, georeferenced):
        if not layers:
            self.iface.messageBar().pushMessage(
                PLUGIN_NAME,
                "No layers found in the MLT file(s).",
                level=Qgis.Warning,
                duration=5,
            )
        else:
            n_files = len(paths)
            file_str = "file" if n_files == 1 else f"{n_files} files"
            coord_msg = " (georeferenced)" if georeferenced else " (raw tile coords)"
            self.iface.messageBar().pushMessage(
                PLUGIN_NAME,
                f"Loaded {len(layers)} layer(s) from {file_str}{coord_msg}",
                level=Qgis.Info,
                duration=5,
            )

    def _report_error(self, exc):
        QgsMessageLog.logMessage(str(exc), PLUGIN_NAME, Qgis.Critical)
        self.iface.messageBar().pushMessage(
            PLUGIN_NAME,
            f"Failed to load MLT file: {exc}",
            level=Qgis.Critical,
            duration=10,
        )
