import cv2
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QImage, QPixmap


def video_thumbnail(self, video_path, width=300, height=300):
    """Extracts the first frame of a video and returns a QPixmap thumbnail."""
    cap = cv2.VideoCapture(video_path)
    success, frame = cap.read()
    cap.release()

    if success:
        frame = cv2.cvtColor(frame, cv2.COLOR_BGR2RGB)
        h, w, ch = frame.shape
        bytes_per_line = ch * w
        qimage = QImage(frame.data, w, h, bytes_per_line, QImage.Format.Format_RGB888)
        pixmap = QPixmap.fromImage(qimage)

        return self.process_image_for_thumbnail(pixmap, width, height)

    else:
        # If thumbnail generation fails, return a default icon
        default_pixmap = QPixmap(width, height)
        default_pixmap.fill(Qt.GlobalColor.gray)
        return default_pixmap
