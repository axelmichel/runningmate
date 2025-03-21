from PyQt6.QtCore import QSize, Qt
from PyQt6.QtGui import QPixmap
from PyQt6.QtWidgets import QHBoxLayout, QLabel, QWidget


class IconLabel(QWidget):

    def __init__(self, image_file_path:str, text:str, alignment=Qt.AlignmentFlag.AlignLeft, size=24, spacing=0):
        super(QWidget, self).__init__()

        icon_size = QSize(size, size)
        layout = QHBoxLayout()
        layout.setContentsMargins(0, 0, 0, 0)

        icon = QLabel()
        pixmap = QPixmap(image_file_path).scaled(
            icon_size, Qt.AspectRatioMode.KeepAspectRatio
        )
        icon.setPixmap(pixmap)
        if alignment == Qt.AlignmentFlag.AlignRight:
            layout.addStretch()
        layout.addWidget(icon)
        layout.addSpacing(spacing)
        layout.addWidget(QLabel(text))
        layout.setAlignment(Qt.AlignmentFlag.AlignVCenter | alignment)
        if alignment == Qt.AlignmentFlag.AlignLeft:
            layout.addStretch()
        self.setLayout(layout)