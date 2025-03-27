import sqlite3

import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations


@pytest.fixture()
def test_db():
    """Set up an in-memory database with migrations applied for testing."""
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    db = DatabaseHandler(conn=conn)
    apply_migrations(db)
    yield db
    db.close()

@pytest.fixture()
def reset_database(test_db):
    """Fully resets the database before each test to ensure a clean state."""
    db = test_db

    db.cursor.execute("PRAGMA foreign_keys = OFF;")

    db.cursor.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
    )
    tables = db.cursor.fetchall()

    for (table_name,) in tables:
        db.cursor.execute(f"DROP TABLE {table_name};")

    db.conn.commit()

    apply_migrations(db)

    db.cursor.execute(
        "PRAGMA foreign_keys = ON;"
    )