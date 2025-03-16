from PyQt6.QtCore import QRect, Qt


def image_thumbnail(pixmap, width, height):
    original_size = pixmap.size()
    if original_size.width() > width and original_size.height() > height:
        crop_size = min(original_size.width(), original_size.height())
        x_offset = (original_size.width() - crop_size) // 2
        y_offset = (original_size.height() - crop_size) // 2
        cropped_pixmap = pixmap.copy(QRect(x_offset, y_offset, crop_size, crop_size))
        return cropped_pixmap.scaled(
            width,
            height,
            Qt.AspectRatioMode.IgnoreAspectRatio,
            Qt.TransformationMode.SmoothTransformation,
        )

    else:
        return pixmap.scaled(
            width,
            height,
            Qt.AspectRatioMode.IgnoreAspectRatio,
            Qt.TransformationMode.SmoothTransformation,
        )
