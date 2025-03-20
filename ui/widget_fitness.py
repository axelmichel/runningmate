from datetime import datetime, timedelta

import pandas as pd
import plotly.graph_objects as go
from PyQt6.QtCore import Qt
from PyQt6.QtWebEngineWidgets import QWebEngineView
from PyQt6.QtWidgets import QLabel, QVBoxLayout, QWidget

from processing.system_settings import (
    ViewMode,
    get_system_background_color,
    mapActivityTypes,
)
from ui.themes import THEME
from utils.translations import _


class FitnessFatigueWidget(QWidget):
    ACTIVITY_WEIGHTS = {
        ViewMode.RUN: 1.0,  # Baseline
        ViewMode.WALK: 0.5,  # Lower impact
        ViewMode.CYCLE: 0.8,  # Moderate impact
    }

    def __init__(self, db_handler, end_date=None, time_range_days=90):
        """
        Initializes the Fitness & Fatigue Chart.

        :param db_handler: Database handler with a `conn` property
        :param time_range_days: Time range in days (default: 90 days)
        """
        super().__init__()
        self.title_label = None
        self.layout = None
        self.web_view = None
        self.db_handler = db_handler
        self.time_range_days = time_range_days
        self.end_date = (
            end_date is not None and datetime.fromtimestamp(end_date) or datetime.now()
        )
        self.start_date = self.end_date - timedelta(days=time_range_days)
        self.init_ui()
        self.plot_fitness_fatigue()

    def init_ui(self):
        """
        Initializes the UI layout and a WebEngineView for displaying the Plotly chart.
        """
        self.layout = QVBoxLayout()
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.setLayout(self.layout)

        self.title_label = QLabel(_("Fitness Trend"))
        self.title_label.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            margin-bottom: 10px;
        """
        )
        self.title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        self.layout.addWidget(self.title_label)

        self.web_view = QWebEngineView()
        self.layout.addWidget(self.web_view)
        self.setLayout(self.layout)

    def fetch_training_load(self, start_date, end_date):
        """
        Fetches daily training load data for the given date range.

        :param start_date: Start date (datetime object)
        :param end_date: End date (datetime object)
        :return: List of (date, TL) tuples
        """
        # 🛠 Ensure dates are converted to datetime
        if isinstance(start_date, str):
            start_date = pd.to_datetime(start_date)

        if isinstance(end_date, str):
            end_date = pd.to_datetime(end_date)

        query = """
            SELECT date, activity_type,
                   AVG(seg_avg_heart_rate * seg_avg_pace / (seg_avg_speed + 1)) as training_load
            FROM activity_details
            JOIN activities ac on activity_details.activity_id = ac.id
            WHERE date BETWEEN ? AND ?
            GROUP BY date, activity_type
            ORDER BY date
        """

        cursor = self.db_handler.conn.cursor()
        cursor.execute(query, (start_date.timestamp(), end_date.timestamp()))
        rows = cursor.fetchall()
        cursor.close()

        training_loads = {}
        for row in rows:
            activity_date, activity_type, tl_value = row
            display_date = pd.to_datetime(activity_date, unit="s").strftime("%Y-%m-%d")

            check_type = mapActivityTypes(activity_type)
            if check_type in self.ACTIVITY_WEIGHTS:
                weighted_tl = (tl_value or 0) * self.ACTIVITY_WEIGHTS[
                    check_type
                ]  # Handle None values safely

                if display_date in training_loads:
                    training_loads[
                        display_date
                    ] += weighted_tl  # Sum loads for multi-sport days
                else:
                    training_loads[display_date] = weighted_tl

        return sorted(training_loads.items())  # Ensure data is sorted by date

    def calculate_fitness_fatigue(self, data, extend_days=42):
        """
        Computes CTL (Fitness), ATL (Fatigue), and Form (TSB).
        If there's not enough historical data, it extends backward safely.

        :param data: List of (date, TL) tuples
        :param extend_days: How many extra days to look back
        :return: Lists of dates, CTL, ATL, and Form values
        """
        if not data:
            return [], [], [], []

        dates, training_loads = zip(*data)
        ctl, atl, form = [], [], []

        # 🚨 Start CTL & ATL from first available TL
        ctl_value = training_loads[0]
        atl_value = training_loads[0]

        for tl in training_loads:
            ctl_value = ctl_value + (tl - ctl_value) / 42  # CTL (Fitness)
            atl_value = atl_value + (tl - atl_value) / 7  # ATL (Fatigue)
            form.append(ctl_value - atl_value)  # Form (Readiness)
            ctl.append(ctl_value)
            atl.append(atl_value)

        # 🔹 Only fetch more data if needed
        if len(dates) < extend_days:
            extended_start_date = pd.to_datetime(dates[0]) - timedelta(days=extend_days)
            extra_data = self.fetch_training_load(extended_start_date, dates[0])

            if extra_data:
                print("🔄 Extending data range...")
                prev_dates, prev_ctl, prev_atl, prev_form = (
                    self.calculate_fitness_fatigue(extra_data, extend_days=0)
                )
                return (
                    prev_dates + dates,
                    prev_ctl + ctl,
                    prev_atl + atl,
                    prev_form + form,
                )

        return dates, ctl, atl, form

    def plot_fitness_fatigue(self):
        """
        Generates a Plotly interactive chart for Fitness & Fatigue.
        """
        system_bg_color = get_system_background_color()
        data = self.fetch_training_load(self.start_date, self.end_date)
        if not data:
            return  # No data available, do nothing
        dates, ctl, atl, form = self.calculate_fitness_fatigue(data)

        fig = go.Figure()

        fig.add_trace(
            go.Scatter(
                x=dates,
                y=ctl,
                mode="lines",
                line={"color": THEME.ACCENT_COLOR_DARK, "width": 2},
                name="Fitness (CTL)",
                showlegend=False,
            )
        )

        fig.add_trace(
            go.Scatter(
                x=dates,
                y=atl,
                mode="lines",
                line={"color": THEME.ACCENT_COLOR_LIGHT, "width": 2},
                name="Fatigue (ATL)",
                showlegend=False,
            )
        )

        fig.add_trace(
            go.Scatter(
                x=dates,
                y=form,
                mode="lines",
                line={"color": THEME.ACCENT_COLOR, "width": 2, "dash": "dash"},
                name="Form (TSB)",
                showlegend=False,
            )
        )

        # Annotations for better clarity
        annotations = [
            {
                "x": dates[-1],
                "y": ctl[-1],
                "xanchor": "left",
                "yanchor": "middle",
                "text": f"Fitness: {ctl[-1]:.1f}",
                "showarrow": False,
                "font": {"color": THEME.ACCENT_COLOR_DARK, "size": 12},
            },
            {
                "x": dates[-1],
                "y": atl[-1],
                "xanchor": "left",
                "yanchor": "middle",
                "text": f"Fatigue: {atl[-1]:.1f}",
                "showarrow": False,
                "font": {"color": THEME.ACCENT_COLOR_LIGHT, "size": 12},
            },
            {
                "x": dates[-1],
                "y": form[-1],
                "xanchor": "left",
                "yanchor": "middle",
                "text": f"Form: {form[-1]:.1f}",
                "showarrow": False,
                "font": {"color": THEME.ACCENT_COLOR, "size": 12},
            },
        ]

        fig.update_layout(
            paper_bgcolor=system_bg_color,
            plot_bgcolor=system_bg_color,
            margin={"l": 0, "r": 0, "t": 0, "b": 0},
            font={"color": "white"},
            xaxis={
                "title": "",
                "showgrid": True,
                "gridcolor": THEME.SYSTEM_BUTTON,
                "gridwidth": 0.5,
            },
            yaxis={
                "showgrid": True,
                "gridcolor": THEME.SYSTEM_BUTTON,
                "gridwidth": 0.5,
            },
            hovermode="x unified",
            annotations=annotations,  # Apply annotations
        )

        # Load the HTML into the WebEngineView
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

        self.web_view.setHtml(html)
