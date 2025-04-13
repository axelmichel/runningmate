from PyQt6.QtWidgets import QButtonGroup, QRadioButton, QVBoxLayout, QWidget

from database.database_handler import DatabaseHandler
from database.user_settings import UserSettings
from processing.system_settings import ViewMode
from utils.translations import _


class ShoeWidget(QWidget):
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
        self.shoe_id = self.activity.get("shoe_id", None)
        self.shoe_list = []
        self._load_shoes()
        if self.activity is None:
            return
        self.init_ui()

    def init_ui(self):
        """
        Initializes the UI layout for the ShoeWidget.
        """
        self.layout = QVBoxLayout()
        self.layout.setContentsMargins(0, 0, 0, 0)
        self.layout.setSpacing(10)
        self.setLayout(self.layout)

        self.radio_group = QButtonGroup(self)
        self.radio_group.buttonClicked.connect(self._on_shoe_selected)

        for shoe in self.shoe_list:
            shoe_id = shoe["id"]
            name = shoe["name"]
            status = shoe["status"]

            if status:
                radio = QRadioButton(f"{name} ({shoe['distance']} km)")
                radio.setProperty("shoe_id", shoe_id)
                self.radio_group.addButton(radio)
                self.layout.addWidget(radio)
                radio.setObjectName(f"shoe_radio_{shoe_id}")

                # Preselect if this is the shoe assigned to the activity
                if shoe_id == self.shoe_id:
                    radio.setChecked(True)
                    font = radio.font()
                    font.setBold(True)
                    radio.setFont(font)
            else:
                if shoe_id == self.shoe_id:
                    label = QRadioButton(
                        f"{name} ({shoe['distance']} km {_('not in use')})"
                    )
                    label.setEnabled(False)
                    font = label.font()
                    font.setItalic(True)
                    label.setFont(font)
                    self.layout.addWidget(label)

    def _on_shoe_selected(self, button):
        """
        Updates the activity's shoe_id in the database when a shoe is selected.
        """
        selected_shoe_id = button.property("shoe_id")
        self._update_shoe(selected_shoe_id)
        self._update_distance(
            selected_shoe_id, self.activity.get("distance", 0.0), True
        )

        if self.shoe_id is not None and self.shoe_id != selected_shoe_id:
            self._update_distance(
                self.shoe_id, self.activity.get("distance", 0.0), False
            )

        self.shoe_id = selected_shoe_id
        for btn in self.radio_group.buttons():
            font = btn.font()
            if btn is button:
                font.setBold(True)
            else:
                font.setBold(False)
            btn.setFont(font)

            shoe_id = btn.property("shoe_id")
            shoe = self.settings.get_shoe(shoe_id)
            if shoe:
                name = shoe.get("name", _("shoe"))
                distance = shoe.get("distance") or 0.0
                btn.setText(f"{name} ({distance} km)")

    def _update_shoe(self, selected_shoe_id: int) -> None:
        if self.activity_type == ViewMode.RUN:
            self.db.update_run(
                {"activity_id": self.activity_id, "shoe_id": selected_shoe_id}
            )
        elif self.activity_type == ViewMode.WALK:
            self.db.update_walking(
                {"activity_id": self.activity_id, "shoe_id": selected_shoe_id}
            )

    def _update_distance(self, shoe_id: int, distance: float, add=True) -> None:
        shoe = self.settings.get_shoe(shoe_id)
        if shoe:
            previous_distance = shoe.get("distance") or 0.0
            if add:
                updated_distance = previous_distance + distance
            else:
                updated_distance = previous_distance - distance
            self.db.update_shoe({"id": shoe_id, "distance": updated_distance})

    def _load_shoes(self) -> None:
        shoes = self.settings.get_shoes()
        for row_index, row_data in enumerate(shoes):
            id = row_data.get("id", None)
            name = row_data.get("name", f"{_("shoe")} {row_index + 1}")
            distance = row_data.get("distance") or 0.0
            status = row_data.get("status", False)
            shoe_enty = {
                "id": id,
                "name": name,
                "distance": round(distance, 2),
                "status": status,
            }
            self.shoe_list.append(shoe_enty)
