from pathlib import Path

import cv2
import numpy as np
from PyQt6.QtGui import QPixmap

from utils.video_thumbnail import video_thumbnail  # update path if needed


def test_video_thumbnail_success(tmp_path: Path, qtbot):
    """Should return a QPixmap from the first frame of a valid video."""
    video_path = tmp_path / "test.avi"

    # Create a 1-frame dummy video
    fourcc = cv2.VideoWriter_fourcc(*'XVID')
    out = cv2.VideoWriter(str(video_path), fourcc, 1.0, (100, 100))
    frame = np.ones((100, 100, 3), dtype=np.uint8) * 128
    out.write(frame)
    out.release()

    # Run thumbnail generator
    thumbnail = video_thumbnail(str(video_path), width=120, height=80)

    # Assertions
    assert isinstance(thumbnail, QPixmap)
    assert not thumbnail.isNull()
    assert thumbnail.width() == 120
    assert thumbnail.height() == 80


def test_video_thumbnail_fallback_on_missing_file(qtbot):
    """Should return a fallback QPixmap when video file does not exist."""
    thumbnail = video_thumbnail("nonexistent/path/to/video.mp4", width=160, height=120)

    assert isinstance(thumbnail, QPixmap)
    assert not thumbnail.isNull()
    assert thumbnail.width() == 160
    assert thumbnail.height() == 120