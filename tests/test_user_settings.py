import pytest

from database.database_handler import DatabaseHandler
from database.user_settings import UserSettings


@pytest.fixture
def user_settings(test_db: DatabaseHandler):
    return UserSettings(test_db)


def test_insert_and_get_user(user_settings: UserSettings):
    user_settings.insert_or_update_user(
        name="Alex", weight=70.5, height=180, hr_min=50, hr_max=190, birthday="1990-01-01"
    )
    user = user_settings.get_user_data()
    assert user is not None
    assert user["name"] == "Alex"
    assert user["weight"] == 70.5
    assert user["hr_max"] == 190


def test_update_user(user_settings: UserSettings):
    user_settings.insert_or_update_user(
        name="Alex", weight=70.5, height=180, hr_min=50, hr_max=190, birthday="1990-01-01"
    )
    user = user_settings.get_user_data()
    user_settings.insert_or_update_user(
        name="Updated Alex", weight=75, height=185, hr_min=55, hr_max=185,
        birthday="1990-01-01", id=user["id"]
    )
    updated_user = user_settings.get_user_data()
    assert updated_user["name"] == "Updated Alex"
    assert updated_user["weight"] == 75


def test_set_heart_rate_zones(user_settings: UserSettings):
    user_settings.insert_or_update_user(
        name="Zoner", weight=80, height=170, hr_min=60, hr_max=180, birthday="1985-12-31"
    )
    user = user_settings.get_user_data()
    user_settings.set_heart_rates_zones(
        id=user["id"], vo2max=50.0, hr_min=60,
        zone1=100, zone2=120, zone3=140, zone4=160, zone5=180
    )
    updated = user_settings.get_user_data()
    assert updated["vo2max"] == 50.0
    assert updated["zone3"] == 140


def test_insert_and_get_shoe(user_settings: UserSettings):
    user_settings.insert_shoe("Nike Air", True)
    shoes = user_settings.get_shoes()
    assert len(shoes) == 1
    assert shoes[0]["name"] == "Nike Air"


def test_insert_and_get_bike(user_settings: UserSettings):
    user_settings.insert_bike("Trek", 9.5, True)
    bikes = user_settings.get_bikes()
    assert len(bikes) == 1
    assert bikes[0]["name"] == "Trek"


def test_update_bike_status(user_settings: UserSettings):
    user_settings.insert_bike("Canyon", 8.0, True)
    user_settings.update_bike_status("Canyon", False)
    bikes = user_settings.get_bikes()
    assert bikes[0]["status"] == 0


def test_update_shoe_status(user_settings: UserSettings):
    user_settings.insert_shoe("Adidas", True)
    user_settings.update_shoe_status("Adidas", False)
    shoes = user_settings.get_shoes()
    assert shoes[0]["status"] == 0


def test_delete_bike(user_settings: UserSettings):
    user_settings.insert_bike("Giant", 10.0, True)
    bike_id = user_settings.get_bikes()[0]["id"]
    user_settings.delete_bike(bike_id)
    assert user_settings.get_bikes() == []


def test_delete_shoe(user_settings: UserSettings):
    user_settings.insert_shoe("Reebok", True)
    shoe_id = user_settings.get_shoes()[0]["id"]
    user_settings.delete_shoe(shoe_id)
    assert user_settings.get_shoes() == []