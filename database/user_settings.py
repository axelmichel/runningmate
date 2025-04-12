from typing import Any, Optional

from database.database_handler import DatabaseHandler
from utils.calculate_age import calculate_age


class UserSettings:
    def __init__(self, db_handler: DatabaseHandler):
        """Initialize the database connection."""
        self.cursor = db_handler.cursor
        self.conn = db_handler.conn

    def insert_or_update_user(
        self,
        name: str,
        weight: float,
        height: int,
        hr_min: int,
        hr_max: int,
        birthday: str,
        id=None,
    ) -> None:
        """
        Inserts or updates user general information.
        """
        age = calculate_age(birthday)

        if id:
            self.cursor.execute(
                "UPDATE users SET name = ?, weight = ?, age = ?, height = ?, hr_min= ?, hr_max = ?, birthday = ? WHERE id = ?",
                (name, weight, age, height, hr_min, hr_max, birthday, id),
            )
        else:
            self.cursor.execute(
                "INSERT INTO users (name, weight, age, height, hr_min, hr_max, birthday) VALUES (?, ?, ?, ?, ?, ?, ?)",
                (name, weight, age, height, hr_min, hr_max, birthday),
            )
        self.conn.commit()

    def set_heart_rates_zones(
        self,
        id: int,
        vo2max: float,
        hr_min: int,
        zone1: int,
        zone2: int,
        zone3: int,
        zone4: int,
        zone5: int,
    ) -> None:
        """
        Inserts or updates hr zones and vo2max.
        """
        if id:
            self.cursor.execute(
                "UPDATE users SET vo2max=?, hr_min=?, zone1=?, zone2=?, zone3=?, zone4=?, zone5=? WHERE id = ?",
                (vo2max, hr_min, zone1, zone2, zone3, zone4, zone5, id),
            )
        self.conn.commit()

    def get_user_data(self) -> Optional[dict[str, any]]:
        """
        Retrieves user data from the database.
        """
        self.cursor.execute("SELECT * FROM users LIMIT 1")
        row = self.cursor.fetchone()
        return dict(row) if row else None

    def insert_shoe(self, name: str, status: bool) -> None:
        """
        Inserts a new shoe into the database.
        """
        self.cursor.execute(
            "INSERT INTO shoes (name, status) VALUES (?, ?)", (name, status)
        )
        self.conn.commit()

    def get_shoes(
        self,
    ) -> list[dict[Any, Any] | dict[str, Any] | dict[str, str] | dict[bytes, bytes]]:
        """
        Retrieves all shoes from the database.
        """
        self.cursor.execute("SELECT id, name, distance, status FROM shoes")
        rows = self.cursor.fetchall()
        return [dict(row) for row in rows]

    def get_shoe(
        self,
        shoe_id: int,
    ) -> Optional[dict[str, Any] | dict[str, str] | dict[bytes, bytes]]:
        """
        Retrieves all shoes from the database.
        """
        self.cursor.execute(
            "SELECT id, name, distance, status FROM shoes WHERE id = ?", (shoe_id,)
        )
        row = self.cursor.fetchone()
        return dict(row) if row else None

    def insert_bike(self, name: str, weight: float, status: bool) -> None:
        """
        Inserts a new bike into the database.
        """
        self.cursor.execute(
            "INSERT INTO bikes (name, weight, status) VALUES (?, ?, ?)",
            (name, status, weight),
        )
        self.conn.commit()

    def get_bikes(
        self,
    ) -> list[dict[Any, Any] | dict[str, Any] | dict[str, str] | dict[bytes, bytes]]:
        """
        Retrieves all bikes from the database.
        """
        self.cursor.execute("SELECT id, name, weight, distance, status FROM bikes")
        rows = self.cursor.fetchall()
        return [dict(row) for row in rows]

    def update_bike_status(self, name: str, status: bool) -> None:
        """
        Updates the status of a bike.
        """
        self.cursor.execute(
            "UPDATE bikes SET status = ? WHERE name = ?", (status, name)
        )
        self.conn.commit()

    def update_shoe_status(self, name: str, status: bool) -> None:
        """
        Updates the status of a shoe.
        """
        self.cursor.execute(
            "UPDATE shoes SET status = ? WHERE name = ?", (status, name)
        )
        self.conn.commit()

    def delete_bike(self, id: int) -> None:
        """
        Deletes a bike from the database.
        """
        self.cursor.execute("DELETE FROM bikes WHERE id = ?", (id,))
        self.conn.commit()

    def delete_shoe(self, id: int) -> None:
        """
        Deletes a shoe from the database.
        """
        self.cursor.execute("DELETE FROM shoes WHERE id = ?", (id,))
        self.conn.commit()
