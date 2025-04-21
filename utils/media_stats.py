import os
from typing import Tuple


def media_stats(folder_path: str) -> Tuple[int, float]:
    """
    Scans a folder and returns the number and total size (in MB) of image and video files.

    Parameters:
        folder_path (str): Path to the folder to scan.

    Returns:
        Tuple[int, float]: A tuple containing the count of media files and their total size in MB.
    """
    image_exits = {".jpg", ".jpeg", ".png", ".gif", ".bmp", ".tiff", ".webp"}
    video_exits = {".mp4", ".avi", ".mov", ".wmv", ".flv", ".mkv", ".webm"}
    media_exits = image_exits.union(video_exits)

    media_count = 0
    total_size_bytes = 0

    for root, _, files in os.walk(folder_path):
        for file in files:
            ext = os.path.splitext(file)[1].lower()
            if ext in media_exits:
                media_count += 1
                total_size_bytes += os.path.getsize(os.path.join(root, file))

    total_size_mb = total_size_bytes / (1024 * 1024)
    return media_count, total_size_mb
