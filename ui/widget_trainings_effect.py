import math
from datetime import datetime, timedelta

from PyQt6.QtCore import Qt
from PyQt6.QtGui import QColor, QMouseEvent, QPainter
from PyQt6.QtWidgets import QLabel, QSizePolicy, QVBoxLayout, QWidget

from processing.system_settings import ViewMode, getAllowedTypes
from ui.themes import THEME
from utils.translations import _


class TrainingEffectWidget(QWidget):
    def __init__(
        self,
        db_handler,
        activity_id,
        activity_type,
        end_date=None,
        chart_width=None,
        time_range_days=90,
    ):
        """
        Initializes the custom Training Effect meter with Garmin-style zones.

        :param db_handler: Database handler with a `conn` property
        :param activity_id: ID of the activity to analyze
        :param activity_type: Type of activity ('cycling' or others)
        :param end_date: End date of the activity (default: now)
        :param time_range_days: Time range in days to calculate baseline TE (default: 90 days)
        """
        super().__init__()
        self.db_handler = db_handler
        self.activity_id = activity_id
        self.activity_type = activity_type
        self.time_range_days = time_range_days
        self.te_value = 1  # Default TE value
        self.end_date = (
            end_date is not None and datetime.fromtimestamp(end_date) or datetime.now()
        )
        self.start_date = self.end_date - timedelta(days=self.time_range_days)
        self.chart_widget = TrainingEffectChart(self, chart_width)
        layout = QVBoxLayout(self)
        layout.setContentsMargins(0, 0, 0, 0)

        title_label = QLabel(_("Training Effect"))
        title_label.setStyleSheet(
            """
            font-size: 13px;
            font-weight: bold;
            margin-bottom: 10px;
        """
        )
        title_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        layout.addWidget(title_label)
        layout.addSpacing(5)
        layout.addWidget(self.chart_widget, stretch=1)
        layout.addSpacing(5)
        self.te_label = QLabel(self)
        self.te_label.setAlignment(Qt.AlignmentFlag.AlignLeft)
        layout.addWidget(self.te_label)

        self.setLayout(layout)

        # ✅ Calculate & Update TE
        self.calculate_and_set_training_effect()

    def calculate_training_effect(self, heart_rate, power, speed, pace, baseline_te):
        """
        Calculates the training effect score with robust error handling.

        :return: Training Effect score (1-12)
        """
        # Ensure all values are valid numbers
        if any(
            x is None or math.isnan(x)
            for x in [heart_rate, power, speed, pace, baseline_te]
        ):
            return 1

        # Use an exponential scale to emphasize intensity
        if self.activity_type == ViewMode.CYCLE:
            current_te = (heart_rate * power) / (speed + 1)
        else:
            current_te = (heart_rate * pace) / (speed + 1)

        # Handle cases where current_te is zero
        if current_te == 0:
            return 1  # Prevent underflow

        # Scale based on HR zone influence (higher weight for high intensity)
        zone_factor = 1 + ((heart_rate - 120) / 20)  # Adjust weighting if needed

        # Prevent baseline_te from being zero
        baseline_te = max(baseline_te, 0.1)

        # Normalize against baseline TE
        effect_score = ((max(current_te * zone_factor, 0)) / (baseline_te + 1)) ** 0.8

        # Ensure TE is in range
        return min(max(round(effect_score * 6), 1), 12)  # Clamp to 1-12 range

    def get_baseline_training_effect(self):
        """
        Computes the baseline TE from past activities within the given time range.

        :return: Baseline TE (float)
        """
        allowed_types = getAllowedTypes(self.activity_type)
        placeholders = ", ".join("?" * len(allowed_types))  # Generate placeholders

        query = f"""
            SELECT seg_avg_heart_rate, seg_avg_power, seg_avg_speed, seg_avg_pace
            FROM activity_details
            JOIN activities ac on activity_details.activity_id = ac.id
            WHERE activity_id != ?
              AND activity_type IN ({placeholders})
              AND date BETWEEN ? AND ?
        """

        cursor = self.db_handler.conn.cursor()
        cursor.execute(
            query,
            (
                self.activity_id,
                *allowed_types,
                self.start_date.timestamp(),
                self.end_date.timestamp(),
            ),
        )
        rows = cursor.fetchall()

        if not rows:
            return 10

        total_te, count = 0, 0
        for row in rows:
            heart_rate, power, speed, pace = row

            if any(x is None or math.isnan(x) for x in row):
                continue  # Ignore invalid data

            speed = max(speed, 0.1)

            if self.activity_type == ViewMode.CYCLE:
                te = (heart_rate * power) / (speed + 1)
            else:
                te = (heart_rate * pace) / (speed + 1)

            if math.isnan(te) or te is None:
                continue  # Ignore invalid TE calculations

            total_te += te
            count += 1

        cursor.close()

        if count == 0:
            return 10

        baseline_te = total_te / count
        return max(baseline_te, 10)

    def calculate_and_set_training_effect(self):
        """
        Calculates and updates the TE based on database values.
        """
        query = """
            SELECT AVG(seg_avg_heart_rate), AVG(seg_avg_power), AVG(seg_avg_speed), AVG(seg_avg_pace)
            FROM activity_details
            WHERE activity_id = ?
        """
        cursor = self.db_handler.conn.cursor()
        cursor.execute(query, (self.activity_id,))
        row = cursor.fetchone()

        if row and all(row):
            heart_rate, power, speed, pace = row
            baseline_te = self.get_baseline_training_effect()
            self.te_value = self.calculate_training_effect(
                heart_rate, power, speed, pace, baseline_te
            )
            self.te_label.setText(
                f"{self.get_te_description()} {_("To fully recover you should rest for {time}.").format(time=self.calculate_recovery_time())}"
            )
            self.chart_widget.set_te_value(self.te_value)

    def get_te_description(self):
        """
        Returns a description of the training effect based on the TE score.
        This is translatable using _().
        """
        descriptions = {
            1: _("Recovery - Minimal training stimulus, good for recovery."),
            2: _(
                "Recovery - Very light effort, maintains fitness but doesn’t improve it."
            ),
            3: _("Base - Light training, improves basic endurance."),
            4: _("Base - Moderate training, beneficial for building endurance."),
            5: _("Base - Strong endurance training, prepares for harder sessions."),
            6: _("Tempo - Improves aerobic capacity and sustainable speed."),
            7: _("Tempo - Effective for increasing endurance performance."),
            8: _("Threshold - High-intensity effort, enhances lactate threshold."),
            9: _(
                "Threshold - Very strong training effect, challenging but sustainable."
            ),
            10: _("VO2 Max - Near-maximal effort, improves oxygen utilization."),
            11: _("VO2 Max - Very high intensity, excellent for performance gains."),
            12: _("Anaerobic - Extreme effort, boosts speed and explosive power."),
        }
        return descriptions.get(self.te_value, _("Unknown Training Effect"))

    def calculate_recovery_time(self):
        """
        Calculates recommended rest time based on TE using an exponential scaling formula.

        :return: Recovery time in hours (rounded), or in days if necessary.
        """
        base_recovery = 6

        growth_factor = 0.28

        recovery_hours = base_recovery * math.exp(growth_factor * self.te_value)

        # Format return value (convert to days if needed)
        if recovery_hours > 24:
            full_days = int(recovery_hours // 24)
            return _("{:d} days").format(full_days)
        else:
            full_hours = int(recovery_hours)
            return _("{:.1f} hours").format(full_hours)


class TrainingEffectChart(QWidget):
    def __init__(self, parent=None, fixed_width=None):
        super().__init__(parent)
        self.te_value = 1  # Default Training Effect Score
        if fixed_width is not None:
            self.setFixedWidth(fixed_width)
            self.setSizePolicy(QSizePolicy.Policy.Fixed, QSizePolicy.Policy.Expanding)
        else:
            self.setSizePolicy(
                QSizePolicy.Policy.Expanding, QSizePolicy.Policy.Expanding
            )

    def set_te_value(self, value):
        """Updates TE value and repaints the chart"""
        self.te_value = value
        self.update()

    def paintEvent(self, event):
        """
        Custom paint event to draw the Garmin-style Training Effect meter.
        """
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        width = self.width()
        height = self.height()
        num_bars = 12  # Scale 1-12
        bar_width = width // num_bars

        for i in range(num_bars):
            if i < 2:
                color = QColor(THEME.MAIN_COLOR_DARK)
            elif i < 4:
                color = QColor(THEME.MAIN_COLOR)
            elif i < 6:
                color = QColor(THEME.MAIN_COLOR_LIGHT)
            elif i < 8:
                color = QColor(THEME.ACCENT_COLOR)
            else:
                color = QColor(THEME.ACCENT_COLOR_LIGHT)

            # Deactivate color for bars not in TE range
            if i >= self.te_value:
                color.setAlpha(20)  # Make inactive bars semi-transparent

            painter.setBrush(color)

            # Calculate bar height for triangular effect
            bar_height = (height // num_bars) * (i + 1)

            # Draw rectangle bars
            painter.drawRect(
                i * bar_width, height - bar_height, bar_width - 2, bar_height
            )

        painter.end()

    def mousePressEvent(self, event: QMouseEvent):
        """
        Allows clicking on bars to adjust the training effect.
        """
        if event.button() == Qt.MouseButton.LeftButton:
            bar_width = self.width() // 12
            clicked_index = event.position().x() // bar_width
            self.te_value = min(max(int(clicked_index) + 1, 1), 12)
            self.update()
