import sqlite3

from database.database_handler import DatabaseHandler
from utils.logger import logger


def get_current_version(db: DatabaseHandler):
    db.cursor.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)"
    )
    db.conn.commit()

    db.cursor.execute("SELECT MAX(version) FROM schema_version")
    result = db.cursor.fetchone()

    return result[0] if result[0] is not None else 0


def apply_migrations(db: DatabaseHandler, custom_migrations=None):
    current_version = get_current_version(db)

    migrations = (
        custom_migrations
        if custom_migrations is not None
        else [
            (
                1,
                """
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
            """,
            ),
            (
                2,
                """
            CREATE TABLE IF NOT EXISTS activities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date_time TEXT NOT NULL,
                distance REAL NOT NULL,
                activity_type TEXT NOT NULL,
                comment TEXT
            );
            """,
            ),
            (
                3,
                """
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
            """,
            ),
            (
                4,
                """
            CREATE TABLE IF NOT EXISTS best_performances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                activity_id INTEGER NOT NULL,
                distance REAL NOT NULL,
                best_time REAL NOT NULL,
                date_time TEXT NOT NULL,
                FOREIGN KEY (activity_id) REFERENCES activities(id)
            );
            """,
            ),
            (
                5,
                [
                    "ALTER TABLE runs ADD COLUMN activity_id INTEGER REFERENCES activities(id)",
                    "CREATE TABLE IF NOT EXISTS media (id INTEGER PRIMARY KEY AUTOINCREMENT, activity_id INTEGER NOT NULL, media_type TEXT NOT NULL, file_path TEXT NOT NULL, FOREIGN KEY (activity_id) REFERENCES activities(id))",
                ],
            ),
            (
                7,
                """
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
         """,
            ),
            (
                8,
                """
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
        """,
            ),
            (
                10,
                [
                    "INSERT INTO activities (date_time, distance, activity_type, comment) SELECT date, distance, activity, comment FROM runs",
                    "UPDATE runs SET activity_id = (SELECT id FROM activities WHERE activities.date_time = runs.date AND activities.distance = runs.distance AND activities.activity_type = runs.activity)",
                    "INSERT INTO media (activity_id, media_type, file_path) SELECT runs.activity_id, 'image', runs.photo FROM runs WHERE runs.photo IS NOT NULL AND runs.photo != ''",
                    "ALTER TABLE runs DROP COLUMN comment",
                    "ALTER TABLE runs DROP COLUMN photo",
                ],
            ),
            (
                11,
                """
                 INSERT INTO cycling (activity_id, date, month, year, start_time, distance, total_time, elevation_gain, avg_speed, avg_power, avg_heart_rate, avg_pace, fastest_pace, slowest_pace, pause, activity, track_img, elevation_img, map_html)
                    SELECT activity_id, date, strftime('%m', date), strftime('%Y', date), start_time, distance, total_time,
                    elevation_gain, avg_speed, avg_power, avg_heart_rate, avg_pace, fastest_pace, slowest_pace, pause,
                    activity, track_img, elevation_img, map_html FROM runs WHERE activity = 'Biking'
                """,
            ),
            (
                12,
                [
                    "ALTER TABLE activities ADD COLUMN title TEXT",
                ],
            ),
            (
                13,
                [
                    "UPDATE activities SET title = activity_type || ' - ' || date_time",
                    "ALTER TABLE activities ADD COLUMN time TEXT",
                    "UPDATE activities SET time = (SELECT start_time FROM runs WHERE runs.activity_id = activities.id)",
                ],
            ),
            (
                14,
                """
            CREATE TABLE IF NOT EXISTS settings (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                age INTEGER CHECK (age >= 0),
                weight REAL CHECK (weight > 0),
                height REAL CHECK (height > 0),
                image TEXT
            );
            """,
            ),
            (
                15,
                """
               CREATE TABLE IF NOT EXISTS shoes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    distance REAL CHECK (distance >= 0),
                    status TEXT,
                    image TEXT
               );
            """,
            ),
            (
                16,
                """
            CREATE TABLE IF NOT EXISTS bikes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                weight REAL CHECK (weight > 0),
                distance REAL CHECK (distance >= 0),
                status TEXT,
                image TEXT
            );
            """,
            ),
            (
                17,
                [
                    "ALTER TABLE runs ADD COLUMN shoe_id INTEGER REFERENCES shoes(id) ON DELETE SET NULL",
                    "ALTER TABLE cycling ADD COLUMN bike_id INTEGER REFERENCES bikes(id) ON DELETE SET NULL",
                    "ALTER TABLE activities DROP COLUMN date_time",
                ],
            ),
            (
                18,
                [
                    "ALTER TABLE best_performances ADD COLUMN activity_type TEXT",
                ],
            ),
            (
                19,
                [
                    "ALTER TABLE activities ADD COLUMN duration",
                    "UPDATE activities SET duration = (SELECT total_time FROM runs WHERE runs.activity_id = activities.id)",
                ],
            ),
            (
                20,
                [
                    "ALTER TABLE activities ADD COLUMN duration_new INTEGER",
                    "UPDATE activities SET duration_new = ((substr(total_time, 1, 2) * 3600) + (substr(total_time, 4, 2) * 60) + (substr(total_time, 7, 2))) FROM runs WHERE activities.id = runs.activity_id",
                    "ALTER TABLE activities DROP COLUMN duration",
                    "ALTER TABLE activities RENAME COLUMN duration_new TO duration",
                ],
            ),
            (
                22,
                [
                    "ALTER TABLE activities ADD COLUMN date_time_new INTEGER",
                    "UPDATE activities SET date_time_new = (SELECT strftime('%s', substr(runs.date, 1, 4) || '-' || substr(runs.date, 6, 2) || '-' || substr(runs.date, 9, 2) || ' ' ||  runs.start_time || ':00') FROM runs WHERE runs.activity_id = activities.id)",
                    "ALTER TABLE activities RENAME COLUMN date_time_new TO date",
                ],
            ),
            (
                23,
                [
                    "ALTER TABLE runs DROP COLUMN date",
                    "ALTER TABLE runs DROP COLUMN month",
                    "ALTER TABLE runs DROP COLUMN year",
                    "ALTER TABLE runs DROP COLUMN start_time",
                    "ALTER TABLE runs DROP COLUMN total_time",
                    "ALTER TABLE runs DROP COLUMN distance",
                    "ALTER TABLE runs DROP COLUMN activity",
                ],
            ),
            (
                24,
                [
                    "ALTER TABLE cycling DROP COLUMN date",
                    "ALTER TABLE cycling DROP COLUMN month",
                    "ALTER TABLE cycling DROP COLUMN year",
                    "ALTER TABLE cycling DROP COLUMN start_time",
                    "ALTER TABLE cycling DROP COLUMN total_time",
                    "ALTER TABLE cycling DROP COLUMN distance",
                    "ALTER TABLE cycling DROP COLUMN activity",
                ],
            ),
            (
                25,
                """
            CREATE TABLE IF NOT EXISTS walking (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                activity_id INTEGER NOT NULL,
                elevation_gain INTEGER NOT NULL,
                avg_speed REAL NOT NULL,
                avg_power INTEGER,
                avg_heart_rate INTEGER,
                avg_pace TEXT,
                fastest_pace TEXT,
                slowest_pace TEXT,
                pause TEXT,
                track_img TEXT,
                elevation_img TEXT,
                map_html TEXT,
                FOREIGN KEY (activity_id) REFERENCES activities(id)
            )
         """,
            ),
            (
                26,
                [
                    "ALTER TABLE walking ADD COLUMN avg_steps INTEGER",
                    "ALTER TABLE cycling ADD COLUMN total_steps INTEGER",
                ],
            ),
            (
                27,
                [
                    "ALTER TABLE cycling DROP COLUMN total_steps",
                    "ALTER TABLE walking ADD COLUMN total_steps INTEGER",
                ],
            ),
            (
                28,
                [
                    "ALTER TABLE activities ADD COLUMN calories INTEGER",
                    "ALTER TABLE settings ADD COLUMN max_heart_rate INTEGER",
                ],
            ),
            (
                29,
                """
            CREATE TABLE IF NOT EXISTS heart_rate_zones (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                zone INTEGER NOT NULL CHECK (zone BETWEEN 1 AND 5),
                name TEXT NOT NULL,
                lower_bound INTEGER NOT NULL,
                upper_bound INTEGER NOT NULL
            )
        """,
            ),
            (
                30,
                [
                    "ALTER TABLE activities ADD COLUMN file_id TEXT",
                ],
            ),
            (
                32,
                """
           UPDATE activities
            SET file_id = CASE 
                WHEN runs.track_img IS NULL OR runs.track_img = '' THEN NULL
                ELSE substr(
                    runs.track_img,
                    instr(runs.track_img, 'images/') + 7,
                    instr(runs.track_img, '_track.png') - (instr(runs.track_img, 'images/') + 7)
                )
            END
            FROM runs  -- Join with runs table
            WHERE runs.activity_id = activities.id
            AND runs.track_img LIKE '%/images/%_track.png%'
         """,
            ),
            (
                33,
                """
           UPDATE activities
            SET file_id = CASE 
                WHEN cycling.track_img IS NULL OR cycling.track_img = '' THEN NULL
                ELSE substr(
                    cycling.track_img,
                    instr(cycling.track_img, 'images/') + 7,
                    instr(cycling.track_img, '_track.png') - (instr(cycling.track_img, 'images/') + 7)
                )
            END
            FROM cycling  -- Join with runs table
            WHERE cycling.activity_id = activities.id
            AND cycling.track_img LIKE '%/images/%_track.png%'
         """,
            ),
            (
                34,
                """
           UPDATE activities
            SET file_id = CASE 
                WHEN walking.track_img IS NULL OR walking.track_img = '' THEN NULL
                ELSE substr(
                    walking.track_img,
                    instr(walking.track_img, 'images/') + 7,
                    instr(walking.track_img, '_track.png') - (instr(walking.track_img, 'images/') + 7)
                )
            END
            FROM walking  -- Join with runs table
            WHERE walking.activity_id = activities.id
            AND walking.track_img LIKE '%/images/%_track.png%'
         """,
            ),
        ]
    )

    for version, queries in migrations:
        if version > current_version:
            print(
                f"📢 Applying Migration {version}..."
            )  # ✅ Show which migration is running

            try:
                if isinstance(queries, list):
                    for query in queries:
                        logger.info(
                            f"Executing query in migration {version}: {query}"
                        )  # ✅ Log SQL query
                        db.cursor.execute(query)
                else:
                    logger.info(
                        f"Executing query in migration {version}: {queries}"
                    )  # ✅ Log SQL query
                    db.cursor.execute(queries)

                logger.info(f"Migration {version} applied successfully.")

                db.cursor.execute(
                    "INSERT INTO schema_version (version) VALUES (?)", (version,)
                )
                db.conn.commit()

            except sqlite3.OperationalError as e:
                logger.warning(
                    f"ERROR in Migration {version}: {e}"
                )  # ✅ Log specific migration failure
                logger.critical("Stopping migrations due to error.")
                break  # Stop further migrations to prevent data corruption
