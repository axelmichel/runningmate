import math
from datetime import datetime, timedelta

import pandas as pd
import plotly.express as px
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QPalette
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWidgets import QApplication, QLabel, QVBoxLayout, QWidget

from database.database_handler import DatabaseHandler
from processing.system_settings import ViewMode
from ui.themes import THEME
from utils.translations import _


class HeartRateTrendWidget(QWidget):
    def __init__(
        self,
        db_handler: DatabaseHandler,
        user_id: int,
        activity_type=ViewMode.RUN,
        center_date_unix=None,
        days: int = 90,
    ):
        super().__init__()
        self.db = db_handler
        self.user_id = user_id
        self.activity_type = activity_type
        self.days = days

        self.end_date = (
            center_date_unix is not None
            and datetime.fromtimestamp(center_date_unix)
            or datetime.now()
        )
        self.start_date = self.end_date - timedelta(days=days)

        self.layout = QVBoxLayout()
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.setLayout(self.layout)

        self.title_label = QLabel(_("Heart Rate Trend"))
        self.title_label.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            margin-bottom: 10px;
        """
        )
        self.title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        self.layout.addWidget(self.title_label)

        # Plotly chart display using QWebEngineView
        self.chart_view = QWebEngineView()
        self.layout.addWidget(self.chart_view)

        # Load initial chart
        self.update_chart()

    def get_system_background_color(self):
        """Fetches the system-defined window background color."""
        app = QApplication.instance() or QApplication([])
        palette = app.palette()
        color = palette.color(QPalette.ColorRole.Window)
        return color.name()

    def _fetch_data(self):
        """Fetches average heart rate data from the database for the given activity type and date range."""
        table = "runs"
        if self.activity_type == ViewMode.CYCLE:
            table = "cycling"
        elif self.activity_type == ViewMode.WALK:
            table = "walking"

        query = f"""
        SELECT avg_heart_rate, activities.date FROM {table}
        JOIN activities ON activities.id = {table}.activity_id
        WHERE date BETWEEN ? AND ?
        ORDER BY date ASC
        """

        df = pd.read_sql_query(
            query,
            self.db.conn,
            params=[int(self.start_date.timestamp()), int(self.end_date.timestamp())],
        )

        df["date"] = pd.to_datetime(df["date"], unit="s").dt.strftime(_("%Y-%m-%d"))

        return df

    def _fetch_heart_rate_zone(self, zone: str):
        """Fetches the ideal heart rate from the user's heart rate zones."""
        query = f"SELECT {zone} FROM users WHERE id = ?"
        cursor = self.db.conn.cursor()
        cursor.execute(query, (self.user_id,))
        result = cursor.fetchone()

        if result and result[0]:
            return result[0]
        return None

    @staticmethod
    def _round_to_nearest_20(value, direction="up"):
        """Round the given value to the nearest multiple of 20."""
        if direction == "up":
            return math.ceil(value / 20) * 20
        else:
            return math.floor(value / 20) * 20

    def update_chart(self):
        """Fetch data and update the Plotly chart with an ideal heart rate reference line."""
        df = self._fetch_data()
        ideal_hr = self._fetch_heart_rate_zone("zone3")
        max_hr = self._fetch_heart_rate_zone("zone5")
        max_zone_hr = max_hr
        min_hr = self._fetch_heart_rate_zone("zone1")
        min_zone_hr = min_hr
        system_bg_color = self.get_system_background_color()

        if not df.empty:
            actual_min = df["avg_heart_rate"].min()
            actual_max = df["avg_heart_rate"].max()

            min_hr = (
                min_hr if min_hr is not None and min_hr < actual_min else actual_min
            )
            max_hr = (
                max_hr if max_hr is not None and max_hr > actual_max else actual_max
            )

        min_hr = self._round_to_nearest_20(min_hr, direction="down")
        max_hr = self._round_to_nearest_20(max_hr, direction="up")

        if df.empty:
            self.chart_view.setHtml(
                "<h3 style='color:white;background:transparent;'>No data available.</h3>"
            )
            return

        fig = px.line(
            df,
            x="date",
            y="avg_heart_rate",
            labels={"date": "", "avg_heart_rate": _("Avg Heart Rate (BPM)")},
            markers=True,
        )
        fig.data[0].update(
            line={"color": THEME.ACCENT_COLOR_LIGHT, "width": 2},
            hovertemplate=f"%{{x}} - {_("BPM")} %{{y}}",
        )

        if ideal_hr:
            fig.add_hline(
                y=ideal_hr,
                line_dash="dash",
                line_color=THEME.MAIN_COLOR_LIGHT,
                annotation_text=f"{_("Ideal HR")}: {ideal_hr} {_("BPM")}",
                annotation_position="top right",
            )
        if max_zone_hr:
            fig.add_hline(
                y=max_zone_hr,
                line_dash="dot",
                line_color=THEME.MAIN_COLOR,
                annotation_text=f"{_("MAX HR")}: {max_zone_hr} {_("BPM")}",
                annotation_position="top right",
            )

        if min_zone_hr:
            fig.add_hline(
                y=min_zone_hr,
                line_dash="dot",
                line_color=THEME.MAIN_COLOR,
                annotation_text=f"{_("MIN HR")}: {min_zone_hr} {_("BPM")}",
                annotation_position="top right",
            )

        window_size = int(self.days / 3)
        df["trend"] = (
            df["avg_heart_rate"].rolling(window=window_size, min_periods=1).mean()
        )

        fig.add_trace(px.line(df, x="date", y="trend", markers=False).data[0])
        fig.data[-1].update(
            mode="lines",
            hoverinfo="skip",
            hovertemplate=None,
            line={
                "shape": "spline",
                "smoothing": 1.3,
                "width": 3,
                "color": THEME.ACCENT_COLOR_DARK,
                "dash": "dot",
            },  # Dashed smooth trend line
            name="Overall Trend",
        )

        fig.update_layout(
            paper_bgcolor=system_bg_color,
            plot_bgcolor=system_bg_color,
            margin={"l": 0, "r": 0, "t": 0, "b": 0},
            font={"color": "white"},
            yaxis={
                "range": [min_hr, max_hr],
                "showgrid": True,
                "gridcolor": THEME.SYSTEM_BUTTON,
                "gridwidth": 0.5,
            },
            xaxis={
                "title": "",
                "showgrid": True,
                "gridcolor": THEME.SYSTEM_BUTTON,
                "gridwidth": 0.5,
            },
        )

        fig.data[0].update(mode="lines+markers")
        fig.data[-1].update(mode="lines")
        fig.update_layout(hovermode="x", template="plotly_dark")

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

        self.chart_view.setHtml(html)
