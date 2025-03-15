import os
import tarfile

from database.database_handler import DatabaseHandler
from processing.tcx_file_parser import TcxFileParser
from utils.logger import logger


class ActivityData:
    def __init__(self, file_path: str, image_path: str, db_handler: DatabaseHandler, parent=None):
        super().__init__()
        self.file_path = file_path
        self.image_path = image_path
        self.db = db_handler
        self.parent = parent

    def get_activity_df(self, activity_id: int):
        """
        Returns the activity data as a DataFrame.
        :param activity_id: The ID of the activity to retrieve.
        """
        activity = self.db.fetch_activity(activity_id)
        if activity:
            file_id = activity.get("file_id")
            if file_id:
                file_path = os.path.join(self.file_path, f"{file_id}.tcx.tar.gz")
                tcx_path = os.path.join(self.file_path, f"{file_id}.tcx")
                if os.path.exists(file_path):
                    proceed = self.unpack_tar(file_path)
                    if proceed:
                        parser = TcxFileParser()
                        parsed, _ = parser.parse_tcx(tcx_path)
                        self.pack_tar(tcx_path)
                        os.remove(tcx_path)
                        return parsed
                elif os.path.exists(tcx_path):
                    parser = TcxFileParser()
                    parsed, _  = parser.parse_tcx(tcx_path)
                    self.pack_tar(tcx_path)
                    os.remove(tcx_path)
                    return parsed
                else:
                    return None

    def pack_tar(self, file_path):
        base_name = os.path.basename(file_path)  # Extract file name
        tar_gz_filename = os.path.join(
            self.file_path, f"{base_name}.tar.gz"
        )  # Output tar.gz file

        with tarfile.open(tar_gz_filename, "w:gz") as tar:
            tar.add(
                file_path, arcname=base_name
            )  # Store file inside archive without full path

    def get_activity_identifier(self, activity_id: int):
        """
        Returns the activity identifier.
        :param activity_id: The ID of the activity to retrieve.
        """
        activity = self.db.fetch_activity(activity_id)
        if activity:
            return activity.get("file_id")
        return None

    def get_activity_type(self, activity_id: int):
        """
        Returns the activity type.
        :param activity_id: The ID of the activity to retrieve.
        """
        activity = self.db.fetch_activity(activity_id)
        if activity:
            return activity.get("activity_type")
        return None

    def save_activity_map(self, activity_id: int, chart_type:str, file_path: str):
       map = self.get_activity_map(activity_id, chart_type)
       if map is not None:
           return map

       self.db.cursor.execute(
           """
            INSERT INTO activity_charts (activity_id, chart_type, file_path) VALUES (?, ?, ?)
           """,
           (activity_id, chart_type, file_path)
         )
       self.db.conn.commit()
       return self.get_activity_map(activity_id, chart_type)

    def get_activity_map(self, activity_id: int, chart_type:str):
        """
        Returns the file path for the activity map.
        :param activity_id: The ID of the activity to retrieve.
        :param chart_type: The type of chart to retrieve.
        """
        self.db.cursor.execute(
            """
            SELECT file_path FROM activity_charts WHERE activity_id = ? AND chart_type = ?
            """,
            (activity_id, chart_type)
        )
        row = self.db.cursor.fetchone()
        if row:
            return dict(row)
        return None

    @staticmethod
    def unpack_tar(file_path):
        """
        Extracts a tar.gz file.
        :param file_path: The path to the tar.gz file.
        """
        extract_dir = os.path.dirname(file_path)  # Extract in the same directory
        try:
            with tarfile.open(file_path, "r:gz") as tar:
                tar.extractall(path=extract_dir)
                return True

        except Exception as e:
            logger.critical(f"Failed to extract tar.gz: {e}")
            return False