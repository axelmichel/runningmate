import math
import os

import folium
import numpy as np
from branca.colormap import linear, LinearColormap

from database.database_handler import DatabaseHandler
from processing.activity_data import ActivityData
from processing.system_settings import ViewMode
from ui.themes import THEME


class TrackMap:
    """
    A class to generate various Folium maps.
    """

    def __init__(self, file_path: str, image_path: str, db_handler: DatabaseHandler, activity_id: int):
        """
        Initializes the TrackMap instance.

        :param file_path: Path to the directory containing the TCX files.
        :param image_path: Path to the directory containing the activity images.
        :param db_handler: DatabaseHandler instance.
        :param activity_id: ID of the activity to generate the map(s) for.
        """
        self.df = None
        self.file_path = file_path
        self.image_path = image_path
        self.db = db_handler
        self.activityData = ActivityData(self.file_path, self.image_path, self.db)
        self.activity_id = activity_id

    def _validate_dataframe(self):
        """Validates that the DataFrame contains necessary columns."""
        required_columns = {"Latitude", "Longitude", "HeartRate", "CleanPace"}
        if not required_columns.issubset(self.df.columns):
            missing = required_columns - set(self.df.columns)
            raise ValueError(f"DataFrame is missing required columns: {missing}")

    def create_map(self, map_type="track"):
        """
        Generates the map and colors the track based on the chosen parameter.
        :param map_type: The type of map to generate. Can be "track", "heart_rate", or "pace".
        """
        self.df = self.activityData.get_activity_df(self.activity_id)
        self._validate_dataframe()

        if map_type == "track":
            activity_map = self._create_track_map()
        else:
            activity_map = self._create_heatmap(map_type)

        return activity_map

    @staticmethod
    def _haversine(point1, point2):
        """
        Computes the Haversine distance between two lat/lon points in kilometers.
        """
        R = 6371  # Earth radius in km
        lat1, lon1 = map(math.radians, point1)
        lat2, lon2 = map(math.radians, point2)

        dlat = lat2 - lat1
        dlon = lon2 - lon1

        a = math.sin(dlat / 2) ** 2 + math.cos(lat1) * math.cos(lat2) * math.sin(dlon / 2) ** 2
        c = 2 * math.atan2(math.sqrt(a), math.sqrt(1 - a))

        return R * c

    def _create_track_map(self):
        """
        Creates a track map with start, end, and segment markers.
        """
        # Get the starting coordinates for the map center
        start_lat, start_lon = self.df.iloc[0]["Latitude"], self.df.iloc[0]["Longitude"]

        # Create the folium map
        activity_map = folium.Map(location=[start_lat, start_lon], zoom_start=14)

        # Extract route coordinates
        route = list(zip(self.df["Latitude"], self.df["Longitude"]))

        # Add the route polyline
        folium.PolyLine(route, color=THEME.MAIN_COLOR, weight=4, opacity=0.7).add_to(activity_map)

        # Add start marker
        folium.Marker(
            route[0],
            icon=folium.DivIcon(
                icon_size=(18, 18),
                icon_anchor=(9, 9),
                html=f'<div style="border: 4px solid {THEME.MAIN_COLOR}; box-sizing: border-box; font-size:1px;'
                     f' background: {THEME.ACCENT_COLOR_LIGHT}; height:18px; width:18px; border-radius: 999px;">'
                     '&nbsp;</div>'
            )
        ).add_to(activity_map)

        folium.Marker(
            route[-1],
            icon=folium.DivIcon(
                icon_size=(18, 18),
                icon_anchor=(9, 9),
                html=f'<div style="border: 4px solid {THEME.MAIN_COLOR}; box-sizing: border-box; font-size:1px;'
                     f' background: {THEME.ACCENT_COLOR_DARK}; height:18px; width:18px; border-radius: 999px;">'
                     '&nbsp;</div>'
            )
        ).add_to(activity_map)

        # Determine the sport type (for segment distance)
        sport_type = self.activityData.get_activity_type(self.activity_id)
        segment_distance = 1 if sport_type in [ViewMode.RUN,
                                               ViewMode.WALK] else 5  # 1K for running/walking, 5K for cycling

        # Add segment markers
        total_distance = 0.0
        over_all_distance = 0.0
        prev_point = route[0]

        for i in range(1, len(route)):
            # Compute distance between consecutive points (approximate)
            distance = self._haversine(prev_point, route[i])
            total_distance += distance
            over_all_distance += distance
            prev_point = route[i]

            # Place a marker every segment_distance (1K or 5K)
            if total_distance >= segment_distance:
                folium.Marker(
                    route[i],
                    icon=folium.DivIcon(
                        icon_size=(25, 25),
                        icon_anchor=(12, 12),
                        html=f'<div style="font-size: 8px; font-weight: bold; box-sizing: border-box; color: #fff; text-align: center;'
                             f' background: {THEME.MAIN_COLOR}; height:25px width:25px; line-height:25px; border-radius: 999px;">'
                             f'{int(over_all_distance)}</div>'
                    )
                ).add_to(activity_map)
                total_distance = 0.0  # Reset for next segment

        # Save the HTML map
        name = self.activityData.get_activity_identifier(self.activity_id)
        html_path = os.path.join(self.image_path, f"{name}_map.html")
        activity_map.save(html_path)

        return self.activityData.save_activity_map(self.activity_id, "track", html_path)

    def _create_heatmap(self, map_type: str):

        df_key = map_type == "heart_rate" and "HeartRate" or "CleanPace"
        df = self.df.dropna(subset=[df_key])
        df = df[np.isfinite(df[df_key])]

        min_value = df[df_key].min()
        max_value = df[df_key].max()

            # Ensure distinct values
        if min_value == max_value:
            min_value -= 0.01  # Avoid zero-range error

        if map_type == "heart_rate":
            color_scale = linear.Spectral_04  # Get the base colormap
            reversed_colors = color_scale.colors[::-1]  # Reverse the color list
            color_scale = LinearColormap(reversed_colors, vmin=min_value, vmax=max_value)
        else:
            color_scale = linear.plasma  # Get the base colormap
            reversed_colors = color_scale.colors[::-1]  # Reverse the color list
            color_scale = LinearColormap(reversed_colors, vmin=min_value, vmax=max_value)

        activity_map = folium.Map(
            location=[df["Latitude"].iloc[0], df["Longitude"].iloc[0]],
            zoom_start=14,
            tiles="cartodbpositron"
        )
        route = list(zip(self.df["Latitude"], self.df["Longitude"]))

        folium.Marker(
            route[0],
            icon=folium.DivIcon(
                icon_size=(18, 18),
                icon_anchor=(9, 9),
                html=f'<div style="border: 4px solid {THEME.MAIN_COLOR}; box-sizing: border-box; font-size:1px;'
                     f' background: {THEME.ACCENT_COLOR_LIGHT}; height:18px; width:18px; border-radius: 999px;">'
                     '&nbsp;</div>'
            )
        ).add_to(activity_map)

        folium.Marker(
            route[-1],
            icon=folium.DivIcon(
                icon_size=(18, 18),
                icon_anchor=(9, 9),
                html=f'<div style="border: 4px solid {THEME.MAIN_COLOR}; box-sizing: border-box; font-size:1px;'
                     f' background: {THEME.ACCENT_COLOR_DARK}; height:18px; width:18px; border-radius: 999px;">'
                     '&nbsp;</div>'
            )
        ).add_to(activity_map)

        for i in range(len(df) - 1):
            coords = [(df["Latitude"].iloc[i], df["Longitude"].iloc[i]),
                      (df["Latitude"].iloc[i + 1], df["Longitude"].iloc[i + 1])]
            color = color_scale(df[df_key].iloc[i])

            folium.PolyLine(coords, color=color, weight=5, opacity=0.8).add_to(activity_map)


        name = self.activityData.get_activity_identifier(self.activity_id)
        html_path = os.path.join(self.image_path, f"{name}_map_{map_type}.html")
        activity_map.save(html_path)
        return self.activityData.save_activity_map(self.activity_id, map_type, html_path)
