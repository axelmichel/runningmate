import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns
from matplotlib.backends.backend_qt5agg import FigureCanvasQTAgg as FigureCanvas
from PyQt6.QtGui import QGuiApplication
from PyQt6.QtWidgets import QSizePolicy

from database.database_handler import DatabaseHandler
from processing.system_settings import getAllowedTypes


class HeatmapCanvas(FigureCanvas):
    """Custom QWidget to display the heatmap in PyQt6."""

    def __init__(self, parent=None):
        self.fig, self.ax = plt.subplots(figsize=(12, 6))
        super().__init__(self.fig)
        self.setSizePolicy(QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding)
        self.updateGeometry()
        self.setParent(parent)  # Store parent for background detection

    def plot_heatmap(self, heatmap_data):
        """Generate and display a heatmap with a fully transparent background while keeping the heatmap visible."""
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
        bg_rgba = (0.055, 0.267, 0.161, 0.2)

        self.fig.patch.set_visible(False)  # Hide figure background
        self.ax.set_facecolor("none")  # Make axis background transparent

        cmap = sns.color_palette("Greens_r", as_cmap=True)  # Normal heatmap colors
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

        self.draw()  # Refresh the canvas


class PlotHeatmap:
    def __init__(self, db_handler: DatabaseHandler, vbox_layout, parent=None):
        super().__init__()
        self.db = db_handler
        self.parent = parent
        self.vbox_layout = vbox_layout

    def get_heatmap(self, activity_type=None, end_date=None):
        """Load activity data, filter by activity type, and update the heatmap."""
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

        # Reindex to enforce 52 weeks & 7 days structure
        heatmap_data = heatmap_data.reindex(
            index=weekdays, columns=week_numbers, fill_value=0
        )
        self.update_ui(heatmap_data)

    def update_ui(self, heatmap_data):
        """Remove old heatmap and add the new one to the layout."""
        for i in reversed(range(self.vbox_layout.count())):
            widget = self.vbox_layout.itemAt(i).widget()
            if widget is not None:
                widget.setParent(None)

        heatmap_canvas = HeatmapCanvas(parent=self.parent)  # ✅ Pass parent widget
        heatmap_canvas.plot_heatmap(heatmap_data)
        self.vbox_layout.addWidget(heatmap_canvas)
