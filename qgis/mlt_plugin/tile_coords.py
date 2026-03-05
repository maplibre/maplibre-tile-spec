"""Parse and prompt for tile z/x/y coordinates."""

import re
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from qgis.PyQt.QtCore import Qt
from qgis.PyQt.QtWidgets import (
    QCheckBox,
    QDialog,
    QDialogButtonBox,
    QFormLayout,
    QHeaderView,
    QLabel,
    QSpinBox,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
)

ZXY = Tuple[int, int, int]

_FNAME_RE = re.compile(r"(\d{1,2})[_\-](\d+)[_\-](\d+)")


def parse_zxy_from_path(file_path: str) -> Optional[ZXY]:
    """Try to extract z/x/y from the file name or parent directories.

    Recognises:
      - 14_8297_10749.mlt        (underscore-separated)
      - 14-8297-10749.mlt        (dash-separated)
      - .../14/8297/10749.mlt    (directory hierarchy)
    """
    p = Path(file_path)

    m = _FNAME_RE.search(p.stem)
    if m:
        return int(m.group(1)), int(m.group(2)), int(m.group(3))

    try:
        y_val = int(p.stem)
        x_val = int(p.parent.name)
        z_val = int(p.parent.parent.name)
        if 0 <= z_val <= 30:
            return z_val, x_val, y_val
    except (ValueError, IndexError):
        pass

    return None


class TileCoordDialog(QDialog):
    """Dialog for a single tile — enter or confirm z/x/y coordinates."""

    def __init__(self, parent=None, initial: Optional[ZXY] = None):
        super().__init__(parent)
        self.setWindowTitle("MLT Tile Coordinates")
        self.setMinimumWidth(320)

        layout = QVBoxLayout(self)

        info = QLabel(
            "Vector tiles use local coordinates. To place features\n"
            "on the map correctly, enter the tile's z/x/y address."
        )
        info.setWordWrap(True)
        layout.addWidget(info)

        form = QFormLayout()

        self.z_spin = QSpinBox()
        self.z_spin.setRange(0, 30)
        self.z_spin.setValue(initial[0] if initial else 14)
        form.addRow("Zoom (z):", self.z_spin)

        self.x_spin = QSpinBox()
        self.x_spin.setRange(0, 2**30 - 1)
        self.x_spin.setValue(initial[1] if initial else 0)
        form.addRow("Column (x):", self.x_spin)

        self.y_spin = QSpinBox()
        self.y_spin.setRange(0, 2**30 - 1)
        self.y_spin.setValue(initial[2] if initial else 0)
        form.addRow("Row (y):", self.y_spin)

        self.tms_check = QCheckBox("TMS y-axis (y=0 at south)")
        self.tms_check.setChecked(True)
        self.tms_check.setToolTip(
            "Check for OpenMapTiles, MBTiles, TileJSON sources.\n"
            "Uncheck for OSM slippy-map / XYZ tiles (y=0 at north)."
        )
        form.addRow("Convention:", self.tms_check)

        layout.addLayout(form)

        buttons = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        skip = buttons.addButton("Skip (raw coords)", QDialogButtonBox.RejectRole)
        skip.clicked.connect(self._on_skip)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

        self._skipped = False

    def _on_skip(self):
        self._skipped = True
        self.reject()

    @property
    def skipped(self) -> bool:
        return self._skipped

    def zxy(self) -> ZXY:
        return self.z_spin.value(), self.x_spin.value(), self.y_spin.value()

    @property
    def tms(self) -> bool:
        return self.tms_check.isChecked()


class MultipleTileCoordDialog(QDialog):
    """Dialog for multiple tiles — shows an editable table of detected z/x/y per file."""

    def __init__(
        self,
        parent=None,
        files_with_coords: Optional[List[Tuple[str, Optional[ZXY]]]] = None,
    ):
        super().__init__(parent)
        self.setWindowTitle("MLT Tile Coordinates")
        self.setMinimumWidth(600)
        self.setMinimumHeight(400)

        self._files = files_with_coords or []

        layout = QVBoxLayout(self)

        detected = sum(1 for _, c in self._files if c is not None)
        total = len(self._files)
        info = QLabel(
            f"{total} tiles selected. Coordinates auto-detected for "
            f"{detected}/{total} files.\n"
            "Edit the table below to correct any values."
        )
        info.setWordWrap(True)
        layout.addWidget(info)

        self.table = QTableWidget(total, 4)
        self.table.setHorizontalHeaderLabels(["File", "z", "x", "y"])
        header = self.table.horizontalHeader()
        header.setSectionResizeMode(0, QHeaderView.Stretch)
        for col in (1, 2, 3):
            header.setSectionResizeMode(col, QHeaderView.ResizeToContents)

        for row, (path, coords) in enumerate(self._files):
            name_item = QTableWidgetItem(Path(path).name)
            name_item.setFlags(name_item.flags() & ~Qt.ItemIsEditable)
            self.table.setItem(row, 0, name_item)

            z_val, x_val, y_val = coords if coords else (0, 0, 0)

            z_spin = QSpinBox()
            z_spin.setRange(0, 30)
            z_spin.setValue(z_val)
            self.table.setCellWidget(row, 1, z_spin)

            x_spin = QSpinBox()
            x_spin.setRange(0, 2**30 - 1)
            x_spin.setValue(x_val)
            self.table.setCellWidget(row, 2, x_spin)

            y_spin = QSpinBox()
            y_spin.setRange(0, 2**30 - 1)
            y_spin.setValue(y_val)
            self.table.setCellWidget(row, 3, y_spin)

        layout.addWidget(self.table)

        self.tms_check = QCheckBox("TMS y-axis (y=0 at south)")
        self.tms_check.setChecked(True)
        self.tms_check.setToolTip(
            "Check for OpenMapTiles, MBTiles, TileJSON sources.\n"
            "Uncheck for OSM slippy-map / XYZ tiles (y=0 at north)."
        )
        layout.addWidget(self.tms_check)

        self.merge_check = QCheckBox("Merge same-named layers across tiles")
        self.merge_check.setChecked(True)
        self.merge_check.setToolTip(
            "When checked, layers with the same name from different tiles\n"
            "are combined into a single QGIS layer for seamless viewing."
        )
        layout.addWidget(self.merge_check)

        buttons = QDialogButtonBox(QDialogButtonBox.Ok | QDialogButtonBox.Cancel)
        skip = buttons.addButton("Skip (raw coords)", QDialogButtonBox.RejectRole)
        skip.clicked.connect(self._on_skip)
        buttons.accepted.connect(self.accept)
        buttons.rejected.connect(self.reject)
        layout.addWidget(buttons)

        self._skipped = False

    def _on_skip(self):
        self._skipped = True
        self.reject()

    @property
    def skipped(self) -> bool:
        return self._skipped

    @property
    def tms(self) -> bool:
        return self.tms_check.isChecked()

    @property
    def merge(self) -> bool:
        return self.merge_check.isChecked()

    def file_coords(self) -> Dict[str, Optional[ZXY]]:
        """Return {file_path: (z, x, y)} for each file."""
        result = {}
        for row, (path, _) in enumerate(self._files):
            z = self.table.cellWidget(row, 1).value()
            x = self.table.cellWidget(row, 2).value()
            y = self.table.cellWidget(row, 3).value()
            result[path] = (z, x, y)
        return result
