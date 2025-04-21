import os


def folder_size(folder_path: str) -> float:
    """
    Calculates the total size of a folder and all its subfolders in megabytes (MB).

    Parameters:
        folder_path (str): The root folder path to calculate size for.

    Returns:
        float: Total size in megabytes (MB).
    """
    total_size = 0
    for dirpath, _, filenames in os.walk(folder_path):
        for file in filenames:
            filepath = os.path.join(dirpath, file)
            if os.path.isfile(filepath):  # Skip if it's not a regular file
                total_size += os.path.getsize(filepath)
    return total_size / (1024 * 1024)