from PyQt6.QtWidgets import QButtonGroup, QRadioButton, QVBoxLayout, QWidget

from database.database_handler import DatabaseHandler
from database.user_settings import UserSettings
from processing.system_settings import ViewMode
from utils.translations import _


class BikeWidget(QWidget):
    def __init__(
        self,
        db: DatabaseHandler,
        user_settings: UserSettings,
        activity: dict,
        activity_type: ViewMode,
        parent=None,
    ):
        super().__init__(parent)
        self.layout = None
        self.radio_group = None
        self.settings = user_settings
        self.db = db
        self.activity = activity
        self.activity_id = activity.get("activity_id", None)
        self.activity_type = activity_type
        self.bike_id = self.activity.get("bike_id", None)
        self.bike_list = []
        self._load_bikes()
        if self.activity is None:
            return
        self.init_ui()

    def init_ui(self):
        """
        Initializes the UI layout for the BikeWidget.
        """
        self.layout = QVBoxLayout()
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.layout.setSpacing(10)
        self.setLayout(self.layout)

        self.radio_group = QButtonGroup(self)
        self.radio_group.buttonClicked.connect(self._on_bike_selected)

        for bike in self.bike_list:
            bike_id = bike["id"]
            name = bike["name"]
            status = bike["status"]

            if status:
                radio = QRadioButton(f"{name} ({bike['distance']} km)")
                radio.setProperty("bike_id", bike_id)
                self.radio_group.addButton(radio)
                self.layout.addWidget(radio)
                radio.setObjectName(f"bike_radio_{bike_id}")

                # Preselect if this is the bike assigned to the activity
                if bike_id == self.bike_id:
                    radio.setChecked(True)
                    font = radio.font()
                    font.setBold(True)
                    radio.setFont(font)
            else:
                if bike_id == self.bike_id:
                    label = QRadioButton(
                        f"{name} ({bike['distance']} km {_('not in use')})"
                    )
                    label.setEnabled(False)
                    font = label.font()
                    font.setItalic(True)
                    label.setFont(font)
                    self.layout.addWidget(label)

    def _on_bike_selected(self, button):
        """
        Updates the activity's bike_id in the database when a bike is selected.
        """
        selected_bike_id = button.property("bike_id")
        self._update_bike(selected_bike_id)
        self._update_distance(
            selected_bike_id, self.activity.get("distance", 0.0), True
        )

        if self.bike_id is not None and self.bike_id != selected_bike_id:
            self._update_distance(
                self.bike_id, self.activity.get("distance", 0.0), False
            )

        self.bike_id = selected_bike_id
        for btn in self.radio_group.buttons():
            font = btn.font()
            if btn is button:
                font.setBold(True)
            else:
                font.setBold(False)
            btn.setFont(font)

            bike_id = btn.property("bike_id")
            bike = self.settings.get_bike(bike_id)
            if bike:
                name = bike.get("name", _("bike"))
                distance = bike.get("distance") or 0.0
                btn.setText(f"{name} ({distance} km)")

    def _update_bike(self, selected_bike_id: int) -> None:
        if self.activity_type == ViewMode.CYCLE:
            self.db.update_bike(
                {"activity_id": self.activity_id, "bike_id": selected_bike_id}
            )

    def _update_distance(self, bike_id: int, distance: float, add=True) -> None:
        bike = self.settings.get_bike(bike_id)
        if bike:
            previous_distance = bike.get("distance") or 0.0
            if add:
                updated_distance = previous_distance + distance
            else:
                updated_distance = previous_distance - distance
            self.db.update_bike({"id": bike_id, "distance": updated_distance})

    def _load_bikes(self) -> None:
        bikes = self.settings.get_bikes()
        for row_index, row_data in enumerate(bikes):
            id = row_data.get("id", None)
            name = row_data.get("name", f"{_("bike")} {row_index + 1}")
            distance = row_data.get("distance") or 0.0
            status = row_data.get("status", False)
            bike_enty = {
                "id": id,
                "name": name,
                "distance": round(distance, 2),
                "status": status,
            }
            self.bike_list.append(bike_enty)
