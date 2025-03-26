import os
import sqlite3
import tarfile
from unittest.mock import MagicMock, patch

import pandas as pd
import pytest

from database.database_handler import DatabaseHandler
from database.migrations import apply_migrations
from processing.activity_data import ActivityData


@pytest.fixture()
def test_db():
    conn = sqlite3.connect(":memory:", check_same_thread=False)
    conn.row_factory = sqlite3.Row
    db = DatabaseHandler(conn=conn)
    apply_migrations(db)
    yield db
    db.close()

@pytest.fixture
def activity_data(tmp_path, test_db):
    return ActivityData(str(tmp_path), str(tmp_path), test_db)

def test_identifier_and_type(activity_data, test_db):
    test_db.conn.execute("INSERT INTO activities (id, file_id, activity_type) VALUES (?, ?, ?)", (1, "abc123", "running"))
    test_db.conn.commit()
    assert activity_data.get_activity_identifier(1) == "abc123"
    assert activity_data.get_activity_type(1) == "running"

def test_save_and_get_activity_map(activity_data, test_db):
    test_db.conn.execute("INSERT INTO activities (id, file_id, activity_type) VALUES (?, ?, ?)", (2, "def456", "cycling"))
    test_db.conn.commit()
    result = activity_data.save_activity_map(2, "path", "/some/path")
    assert isinstance(result, dict)
    assert result["file_path"] == "/some/path"

def test_unpack_tar_creates_file(tmp_path):
    tar_path = tmp_path / "test.tar.gz"
    inside = tmp_path / "inside.txt"
    inside.write_text("hello")
    with tarfile.open(tar_path, "w:gz") as tar:
        tar.add(inside, arcname="inside.txt")
    os.remove(inside)
    assert ActivityData.unpack_tar(str(tar_path))
    assert (tmp_path / "inside.txt").exists()

@patch("processing.activity_data.TcxFileParser")
def test_get_activity_df_with_tcx(mock_parser_class, activity_data, test_db, tmp_path):
    test_db.conn.execute("INSERT INTO activities (id, file_id, activity_type) VALUES (?, ?, ?)", (3, "datafile", "running"))
    test_db.conn.commit()
    tcx_path = tmp_path / "datafile.tcx"
    tcx_path.write_text("<tcx>dummy</tcx>")
    tar_path = tmp_path / "datafile.tcx.tar.gz"
    with tarfile.open(tar_path, "w:gz") as tar:
        tar.add(tcx_path, arcname="datafile.tcx")
    os.remove(tcx_path)
    mock_parser = MagicMock()
    mock_parser.parse_tcx.return_value = (pd.DataFrame([{"time": 1, "value": 42}]), None)
    mock_parser_class.return_value = mock_parser
    df = activity_data.get_activity_df(3)
    assert isinstance(df, pd.DataFrame)
    assert not df.empty