import os

import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns
from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg as FigureCanvas
from PyQt6.QtGui import QGuiApplication
from PyQt6.QtWidgets import QSizePolicy

from database.database_handler import DatabaseHandler
from processing.system_settings import getAllowedTypes

# Cache directory for storing heatmap images
CACHE_DIR = os.path.expanduser("~/RunningData/temp")
if not os.path.exists(CACHE_DIR):
    os.makedirs(CACHE_DIR)


class HeatmapCanvas(FigureCanvas):
    """Custom QWidget to generate and save heatmaps in PyQt6."""

    def __init__(self, parent=None):
        self.fig, self.ax = plt.subplots(figsize=(12, 6))
        super().__init__(self.fig)
        self.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.updateGeometry()
        self.setParent(parent)  # Store parent for background detection

    def plot_heatmap(self, heatmap_data, activity_type, save_path):
        """Generate and save a heatmap only if required."""
        self.ax.clear()  # Clear previous plot

        num_weeks = len(heatmap_data.columns)
        num_days = len(heatmap_data.index)

        cell_size = 0.2  # Adjust to make it even smaller
        max_height = 2.0  # Limit height to prevent it from taking up too much space

        fig_width = num_weeks * cell_size
        fig_height = min(num_days * cell_size, max_height)

        self.fig.set_size_inches(fig_width, fig_height)
        mask = heatmap_data == 0
        line_color = QGuiApplication.palette().window().color().name()
        bg_rgba = (0.5, 0.5, 0.5, 0.1)

        self.fig.patch.set_visible(False)  # Hide figure background
        self.ax.set_facecolor("none")  # Make axis background transparent

        cmap = sns.color_palette("rocket", as_cmap=True)
        cmap = cmap.with_extremes(bad=bg_rgba)

        sns.heatmap(
            heatmap_data,
            cmap=cmap,
            linewidths=0.5,
            linecolor=line_color,
            cbar=False,  # No color legend
            ax=self.ax,
            square=True,
            mask=mask,
            xticklabels=False,  # No labels
            yticklabels=False,
        )

        self.ax.set_xticks([])
        self.ax.set_yticks([])
        self.ax.set_xlabel("")
        self.ax.set_ylabel("")
        self.ax.set_title("")

        for _, spine in self.ax.spines.items():
            spine.set_visible(False)  # Remove all borders

        self.fig.savefig(
            save_path, dpi=300, bbox_inches="tight", pad_inches=0.0, transparent=True
        )
        plt.close(self.fig)
        return save_path


class PlotHeatmap:
    def __init__(self, db_handler: DatabaseHandler, parent=None):
        super().__init__()
        self.db = db_handler
        self.parent = parent

    def get_heatmap(self, activity_type=None, end_date=None, redraw=False):
        """
        Load activity data, filter by activity type, and return the heatmap image path.

        :param activity_type: str, the type of activity (e.g., "Run", "Cycle", etc.)
        :param end_date: datetime, the end date for data selection (defaults to today)
        :param redraw: bool, whether to force a redraw of the heatmap (default: False)
        :return: str, path to the saved heatmap image
        """

        # ✅ Define the save path for this activity type
        save_path = os.path.join(CACHE_DIR, f"heatmap_{activity_type}.png")

        # ✅ Check if we need to redraw or if an existing image can be used
        if not redraw and os.path.exists(save_path):
            return save_path  # ✅ Use cached image if it exists

        if end_date is None:
            end_date = pd.Timestamp.today()  # Default to today

        start_date = end_date - pd.Timedelta(days=365)

        query = f"""
            SELECT duration, activity_type, date
            FROM activities
            WHERE date >= {start_date.timestamp()}
              AND date <= {end_date.timestamp()}
            ORDER BY date ASC;
        """

        df = pd.read_sql(query, self.db.conn)

        if activity_type is not None and activity_type != "All":
            allowed_types = getAllowedTypes(activity_type)
            df = df[df["activity_type"].isin(allowed_types)]

        # Convert timestamp to pandas datetime format
        df["date"] = pd.to_datetime(df["date"], unit="s")

        # Extract week number and weekday
        df["weekday"] = df["date"].dt.weekday  # Monday=0, Sunday=6
        df["week"] = df["date"].dt.strftime("%Y-%W")  # Format as 'YYYY-WW'

        # Ensure full 52 weeks on X-axis
        week_numbers = pd.date_range(
            start=start_date, end=end_date, freq="W-MON"
        ).strftime("%Y-%W")

        # Ensure full 7 days (Monday-Sunday) on Y-axis
        weekdays = list(range(7))  # 0=Monday, 6=Sunday

        # Aggregate total duration per day
        if not df.empty:
            heatmap_data = (
                df.groupby(["weekday", "week"])["duration"].sum().unstack(fill_value=0)
            )
        else:
            heatmap_data = pd.DataFrame(
                0, index=weekdays, columns=week_numbers
            )  # Create empty grid

        heatmap_data = heatmap_data.fillna(0)
        if heatmap_data.isnull().values.all():
            heatmap_data.iloc[:, :] = 0  # Replace all NaNs with zeros

        # Reindex to enforce 52 weeks & 7 days structure
        heatmap_data = heatmap_data.reindex(
            index=weekdays, columns=week_numbers, fill_value=0
        )

        if heatmap_data.max().max() == 0:
            heatmap_data = heatmap_data.astype(float)
            heatmap_data.iloc[0, 0] = 1e-6

        heatmap_canvas = HeatmapCanvas(parent=self.parent)  # ✅ Pass parent widget
        return heatmap_canvas.plot_heatmap(heatmap_data, activity_type, save_path)

    @staticmethod
    def clear_heatmaps():
        """Delete all cached heatmap images."""
        for file in os.listdir(CACHE_DIR):
            if file.startswith("heatmap_") and file.endswith(".png"):
                os.remove(os.path.join(CACHE_DIR, file))
