import sqlite3
from unittest.mock import patch

import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations, get_current_version


@pytest.fixture()
def test_db():
    """Setup an in-memory database for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)  # ✅ Inject test connection
    apply_migrations(db)  # ✅ Run migrations using DatabaseHandler
    yield db
    db.close()  # ✅ Cleanup


def test_apply_migrations(test_db):
    """Test that migrations apply correctly in an isolated in-memory database."""
    db = test_db

    # ✅ Check that schema_version table exists
    db.cursor.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'"
    )
    assert db.cursor.fetchone() is not None, "schema_version table should exist"

    # ✅ Check that all expected tables exist
    expected_tables = ["runs", "activities"]
    for table in expected_tables:
        db.cursor.execute(
            f"SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'"
        )
        assert db.cursor.fetchone() is not None, f"Table {table} should exist"


def test_get_current_version(test_db):
    """Ensure that get_current_version correctly retrieves the schema version."""
    db = test_db

    # ✅ Ensure the table is empty before inserting
    db.cursor.execute("DELETE FROM schema_version")  # Clear previous entries

    # ✅ Insert schema version safely
    db.cursor.execute("INSERT INTO schema_version (version) VALUES (3)")
    db.conn.commit()

    assert get_current_version(db) == 3, (
        "get_current_version should return the latest version"
    )


def test_migration_failure_handling(test_db, caplog):
    """Test that a failed migration stops further migrations and logs an error, without popups."""
    db = test_db

    # ✅ Define a faulty migration that will cause an OperationalError
    faulty_migrations = [
        (
            99,
            "ALTER TABLE non_existent_table ADD COLUMN fake_column TEXT",
        ),  # Intentional failure
        (
            100,
            "CREATE TABLE should_not_execute (id INTEGER)",
        ),  # This should NOT execute
    ]

    # ✅ Ensure previous migration version exists
    db.cursor.execute("INSERT INTO schema_version (version) VALUES (98)")
    db.conn.commit()

    # ✅ Patch QMessageBox.exec() to prevent popups during the test
    with (
        patch("utils.logger.QMessageBox.exec", return_value=None),
        caplog.at_level("WARNING"),
    ):
        apply_migrations(db, custom_migrations=faulty_migrations)

    # ✅ Verify that the error message was logged
    assert any(
        "ERROR in Migration 99" in message for message in caplog.text.split("\n")
    ), "Migration failure should be logged as a warning."
    assert any(
        "Stopping migrations due to error." in message
        for message in caplog.text.split("\n")
    ), "Migrations should stop after an error."

    # ✅ Verify that migration 100 was NOT applied
    db.cursor.execute("SELECT version FROM schema_version WHERE version = 100")
    assert db.cursor.fetchone() is None, "Migration 100 should not be applied."
