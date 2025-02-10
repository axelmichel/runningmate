from database.database_handler import DatabaseHandler


def get_current_version(db: DatabaseHandler):
    db.cursor.execute("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)")
    db.conn.commit()

    db.cursor.execute("SELECT MAX(version) FROM schema_version")
    result = db.cursor.fetchone()

    return result[0] if result[0] is not None else 0


def apply_migrations(db: DatabaseHandler):
    current_version = get_current_version(db)

    migrations = [
        (1, """
            CREATE TABLE IF NOT EXISTS runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                month TEXT,
                year TEXT,
                start_time TEXT NOT NULL,
                distance REAL NOT NULL,
                total_time TEXT NOT NULL,
                elevation_gain INTEGER NOT NULL,
                avg_speed REAL NOT NULL,
                avg_steps INTEGER,
                total_steps INTEGER,
                avg_power INTEGER,
                avg_heart_rate INTEGER,
                avg_pace TEXT,
                fastest_pace TEXT,
                slowest_pace TEXT,
                pause TEXT,
                activity TEXT NOT NULL,
                track_img TEXT,
                elevation_img TEXT,
                map_html TEXT,
                comment TEXT,
                photo TEXT
            );
        """),
        (2, """
            CREATE TABLE IF NOT EXISTS activities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date_time TEXT NOT NULL,
                distance REAL NOT NULL,
                activity_type TEXT NOT NULL,
                comment TEXT
            );
        """)
    ]

    for version, queries in migrations:
        if version > current_version:
            if isinstance(queries, list):
                for query in queries:
                    db.cursor.execute(query)
            else:
                db.cursor.execute(queries)

            db.cursor.execute("INSERT INTO schema_version (version) VALUES (?)", (version,))
            db.conn.commit()
