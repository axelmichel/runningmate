import sqlite3
import pytest
from database.migrations import apply_migrations, get_current_version
from database.database_handler import DatabaseHandler

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
    db.cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='schema_version'")
    assert db.cursor.fetchone() is not None, "schema_version table should exist"

    # ✅ Check that all expected tables exist
    expected_tables = ["runs", "activities"]
    for table in expected_tables:
        db.cursor.execute(f"SELECT name FROM sqlite_master WHERE type='table' AND name='{table}'")
        assert db.cursor.fetchone() is not None, f"Table {table} should exist"

def test_get_current_version(test_db):
    """Ensure that get_current_version correctly retrieves the schema version."""
    db = test_db

    # ✅ Ensure the table is empty before inserting
    db.cursor.execute("DELETE FROM schema_version")  # Clear previous entries

    # ✅ Insert schema version safely
    db.cursor.execute("INSERT INTO schema_version (version) VALUES (3)")
    db.conn.commit()

    assert get_current_version(db) == 3, "get_current_version should return the latest version"