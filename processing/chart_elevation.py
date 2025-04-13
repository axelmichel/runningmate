import math

import numpy as np
import plotly.graph_objects as go
from PyQt6.QtGui import QPalette
from PyQt6.QtWidgets import QApplication

from database.database_handler import DatabaseHandler
from processing.activity_data import ActivityData
from processing.system_settings import ViewMode
from ui.themes import THEME
from utils.translations import _


class ElevationChart:
    """
    Generate an elevation chart for a given activity.
    """

    def __init__(
        self,
        file_path: str,
        image_path: str,
        db_handler: DatabaseHandler,
        activity_id: int,
        activity_type=ViewMode.RUN,
    ):
        """
        Initializes the ElevationChart instance.

        :param file_path: Path to the directory containing the TCX files.
        :param image_path: Path to the directory containing the activity images.
        :param db_handler: DatabaseHandler instance.
        :param activity_id: ID of the activity to generate the map(s) for.
        :param activity_type: Type of activity (e.g., RUN, BIKE).
        """
        self.df = None
        self.file_path = file_path
        self.image_path = image_path
        self.db = db_handler
        self.activityData = ActivityData(self.file_path, self.image_path, self.db)
        self.activity_id = activity_id
        self.activity_type = activity_type

    def _validate_dataframe(self):
        """Validates that the DataFrame contains necessary columns."""
        required_columns = {"Latitude", "Longitude", "Elevation", "DistanceInKm"}
        if not required_columns.issubset(self.df.columns):
            missing = required_columns - set(self.df.columns)
            raise ValueError(f"DataFrame is missing required columns: {missing}")

    def create_chart(self):
        """
        Generates the map and colors the track based on the chosen parameter.
        """
        self.df = self.activityData.get_activity_df(self.activity_id)
        if self.df is None or self.df.empty:
            return
        self._validate_dataframe()
        return self._create_chart()

    @staticmethod
    def _round_to_nearest_20(value, direction="up"):
        """Round the given value to the nearest multiple of 20."""
        if direction == "up":
            return math.ceil(value / 20) * 20
        else:
            return math.floor(value / 20) * 20

    @staticmethod
    def _get_system_background_color():
        """Fetches the system-defined window background color."""
        app = QApplication.instance() or QApplication([])
        palette = app.palette()
        color = palette.color(QPalette.ColorRole.Window)
        return color.name()

    def _create_chart(self):
        self.df["Elevation"] = self.df["Elevation"].round().astype(int)
        self.df["DistanceInKm"] = self.df["DistanceInKm"].round(2)
        distance_max = self.df["DistanceInKm"].max(skipna=True)
        max_km = math.ceil(distance_max) if not np.isnan(distance_max) else 0
        if max_km == 0:
            return

        tick = 5 if self.activity_type == ViewMode.CYCLE else 1
        tick_vals = np.arange(0, max_km + tick, tick)
        tick_text = ["" if i == 0 else str(v) for i, v in enumerate(tick_vals)]

        actual_min = self._round_to_nearest_20(
            self.df["Elevation"].min(), direction="down"
        )
        actual_max = self._round_to_nearest_20(
            self.df["Elevation"].max(), direction="up"
        )
        system_bg_color = self._get_system_background_color()

        fig = go.Figure()

        fig.add_trace(
            go.Scatter(
                x=self.df["DistanceInKm"],
                y=self.df["Elevation"],
                mode="lines",
                line={"color": THEME.ACCENT_COLOR_LIGHT, "width": 2, "shape": "spline"},
                hovertemplate=f"{_('Distance (km)')}: %{{x}}<br>{_('Elevation (m)')}: %{{y}}<extra></extra>",
            )
        )
        fig.update_layout(
            paper_bgcolor=system_bg_color,
            plot_bgcolor=system_bg_color,
            margin={"l": 0, "r": 0, "t": 0, "b": 0},
            font={"color": "white"},
            yaxis={
                "range": [actual_min, actual_max],
                "showgrid": True,
                "showline": False,
                "zeroline": False,
                "gridcolor": THEME.SYSTEM_BUTTON,
                "gridwidth": 0.5,
                "dtick": 50,
            },
            xaxis={
                "range": [0, max_km],
                "title": "",
                "showgrid": True,
                "showline": False,
                "zeroline": False,
                "gridcolor": THEME.SYSTEM_BUTTON,
                "gridwidth": 0.5,
                "tickvals": tick_vals,
                "ticktext": tick_text,
            },
        )
        html = f"""
                   <html>
                   <head>
                       <style>
                           html,body {{
                               background-color: {system_bg_color};
                               margin: 0;
                               padding: 0;
                           }}
                       </style>
                   </head>
                   <body>
                       {fig.to_html(include_plotlyjs="cdn", full_html=False, config={"displayModeBar": False})}
                   </body>
                   </html>
                   """

        return html
