from database.database_handler import connect


def get_current_version():
    conn, cursor = connect()
    cursor.execute("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)")
    cursor.execute("SELECT MAX(version) FROM schema_version")
    result = cursor.fetchone()
    conn.close()
    return result[0] if result[0] is not None else 0


def apply_migrations():
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
        """),
        (3, """
            CREATE TABLE IF NOT EXISTS run_details (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                activity_id INTEGER NOT NULL,
                segment_number INTEGER NOT NULL,
                heart_rate INTEGER,
                speed REAL,
                pace REAL,
                pause_time REAL,
                FOREIGN KEY (activity_id) REFERENCES activities(id)
            );
        """),
        (4, """
            CREATE TABLE IF NOT EXISTS best_performances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                activity_id INTEGER NOT NULL,
                distance REAL NOT NULL,
                best_time REAL NOT NULL,
                date_time TEXT NOT NULL,
                FOREIGN KEY (activity_id) REFERENCES activities(id)
            );
        """),
        (5, [
            "ALTER TABLE runs ADD COLUMN activity_id INTEGER REFERENCES activities(id)",
            "CREATE TABLE IF NOT EXISTS media (id INTEGER PRIMARY KEY AUTOINCREMENT, activity_id INTEGER NOT NULL, media_type TEXT NOT NULL, file_path TEXT NOT NULL, FOREIGN KEY (activity_id) REFERENCES activities(id))"
        ]),
        (7, """
            CREATE TABLE IF NOT EXISTS cycling (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                activity_id INTEGER NOT NULL,
                date TEXT NOT NULL,
                month TEXT,
                year TEXT,
                start_time TEXT NOT NULL,
                distance REAL NOT NULL,
                total_time TEXT NOT NULL,
                elevation_gain INTEGER NOT NULL,
                avg_speed REAL NOT NULL,
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
                FOREIGN KEY (activity_id) REFERENCES activities(id)
            )
         """),
        (8, """
                CREATE TABLE IF NOT EXISTS cycling_details (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    activity_id INTEGER NOT NULL,
                    segment_number INTEGER NOT NULL,
                    heart_rate INTEGER,
                    avg_power INTEGER,
                    speed REAL,
                    pace REAL,
                    pause_time REAL,
                    FOREIGN KEY (activity_id) REFERENCES activities(id)
                );
            """),
        (10, [
            "INSERT INTO activities (date_time, distance, activity_type, comment) SELECT date, distance, activity, comment FROM runs",
            "UPDATE runs SET activity_id = (SELECT id FROM activities WHERE activities.date_time = runs.date AND activities.distance = runs.distance AND activities.activity_type = runs.activity)",
            "INSERT INTO media (activity_id, media_type, file_path) SELECT runs.activity_id, 'image', runs.photo FROM runs WHERE runs.photo IS NOT NULL AND runs.photo != ''",
            "ALTER TABLE runs DROP COLUMN comment",
            "ALTER TABLE runs DROP COLUMN photo"
        ]),
        (11, """ 
             INSERT INTO cycling (activity_id, date, month, year, start_time, distance, total_time, elevation_gain, avg_speed, avg_power, avg_heart_rate, avg_pace, fastest_pace, slowest_pace, pause, activity, track_img, elevation_img, map_html)
                SELECT activity_id, date, strftime('%m', date), strftime('%Y', date), start_time, distance, total_time,
                elevation_gain, avg_speed, avg_power, avg_heart_rate, avg_pace, fastest_pace, slowest_pace, pause,
                activity, track_img, elevation_img, map_html FROM runs WHERE activity = 'Biking'
            """),
    ]

    current_version = get_current_version()
    conn, cursor = connect()

    for version, queries in migrations:
        if version > current_version:
            if isinstance(queries, list):  # ✅ If multiple queries, loop through them
                for query in queries:
                    cursor.execute(query)
            else:
                cursor.execute(queries)
            cursor.execute("INSERT INTO schema_version (version) VALUES (?)", (version,))
            conn.commit()

    conn.close()