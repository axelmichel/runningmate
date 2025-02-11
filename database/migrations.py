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
        (12, [
            "ALTER TABLE activities ADD COLUMN title TEXT",
        ]),
        (13, [
            "UPDATE activities SET title = activity_type || ' - ' || date_time",
            "ALTER TABLE activities ADD COLUMN time TEXT",
            "UPDATE activities SET time = (SELECT start_time FROM runs WHERE runs.activity_id = activities.id)"
        ]),
        (14, """
            CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                age INTEGER CHECK (age >= 0),
                weight REAL CHECK (weight > 0),
                height REAL CHECK (height > 0),
                image TEXT
            );
            """),
        (15, """
               CREATE TABLE IF NOT EXISTS shoes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    distance REAL CHECK (distance >= 0),
                    status TEXT,
                    image TEXT
               );
            """),
        (16, """
            CREATE TABLE IF NOT EXISTS bikes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,  
                weight REAL CHECK (weight > 0),
                distance REAL CHECK (distance >= 0),
                status TEXT,
                image TEXT
            );
            """),
        (17, [
            "ALTER TABLE runs ADD COLUMN shoe_id INTEGER REFERENCES shoes(id) ON DELETE SET NULL",
            "ALTER TABLE cycling ADD COLUMN bike_id INTEGER REFERENCES bikes(id) ON DELETE SET NULL"
        ]),
        (18, [
            "ALTER TABLE best_performances ADD COLUMN activity_type TEXT",
        ]),
        (19, [
            "ALTER TABLE activities ADD COLUMN duration",
            "UPDATE activities SET duration = (SELECT total_time FROM runs WHERE runs.activity_id = activities.id)"
        ]),
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
