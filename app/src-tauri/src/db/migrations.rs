//! SQLite schema migrations.
//!
//! Versions 1–55 are copied verbatim from the Python app (`database/migrations.py`, generated
//! from the source rather than retyped) so both apps can open the same `running_data.db`.
//! Applied versions are recorded in `schema_version`, exactly as the Python app does. New
//! migrations go at the end with a higher version number and must only add to the schema.

/// `(version, statements)`, applied in order for every version above the current one.
pub const MIGRATIONS: &[(i64, &[&str])] = &[
    (
        1,
        &[r#"CREATE TABLE IF NOT EXISTS runs (
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
);"#],
    ),
    (
        2,
        &[r#"CREATE TABLE IF NOT EXISTS activities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date_time TEXT NOT NULL,
    distance REAL NOT NULL,
    activity_type TEXT NOT NULL,
    comment TEXT
);"#],
    ),
    (
        3,
        &[r#"CREATE TABLE IF NOT EXISTS run_details (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id INTEGER NOT NULL,
    segment_number INTEGER NOT NULL,
    heart_rate INTEGER,
    speed REAL,
    pace REAL,
    pause_time REAL,
    FOREIGN KEY (activity_id) REFERENCES activities(id)
);"#],
    ),
    (
        4,
        &[r#"CREATE TABLE IF NOT EXISTS best_performances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id INTEGER NOT NULL,
    distance REAL NOT NULL,
    best_time REAL NOT NULL,
    date_time TEXT NOT NULL,
    FOREIGN KEY (activity_id) REFERENCES activities(id)
);"#],
    ),
    (
        5,
        &[
            r#"ALTER TABLE runs ADD COLUMN activity_id INTEGER REFERENCES activities(id)"#,
            r#"CREATE TABLE IF NOT EXISTS media (id INTEGER PRIMARY KEY AUTOINCREMENT, activity_id INTEGER NOT NULL, media_type TEXT NOT NULL, file_path TEXT NOT NULL, FOREIGN KEY (activity_id) REFERENCES activities(id))"#,
        ],
    ),
    (
        7,
        &[r#"CREATE TABLE IF NOT EXISTS cycling (
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
)"#],
    ),
    (
        8,
        &[r#"CREATE TABLE IF NOT EXISTS cycling_details (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    activity_id INTEGER NOT NULL,
    segment_number INTEGER NOT NULL,
    heart_rate INTEGER,
    avg_power INTEGER,
    speed REAL,
    pace REAL,
    pause_time REAL,
    FOREIGN KEY (activity_id) REFERENCES activities(id)
);"#],
    ),
    (
        10,
        &[
            r#"INSERT INTO activities (date_time, distance, activity_type, comment) SELECT date, distance, activity, comment FROM runs"#,
            r#"UPDATE runs SET activity_id = (SELECT id FROM activities WHERE activities.date_time = runs.date AND activities.distance = runs.distance AND activities.activity_type = runs.activity)"#,
            r#"INSERT INTO media (activity_id, media_type, file_path) SELECT runs.activity_id, 'image', runs.photo FROM runs WHERE runs.photo IS NOT NULL AND runs.photo != ''"#,
            r#"ALTER TABLE runs DROP COLUMN comment"#,
            r#"ALTER TABLE runs DROP COLUMN photo"#,
        ],
    ),
    (
        11,
        &[
            r#"INSERT INTO cycling (activity_id, date, month, year, start_time, distance, total_time, elevation_gain, avg_speed, avg_power, avg_heart_rate, avg_pace, fastest_pace, slowest_pace, pause, activity, track_img, elevation_img, map_html)
   SELECT activity_id, date, strftime('%m', date), strftime('%Y', date), start_time, distance, total_time,
   elevation_gain, avg_speed, avg_power, avg_heart_rate, avg_pace, fastest_pace, slowest_pace, pause,
   activity, track_img, elevation_img, map_html FROM runs WHERE activity = 'Biking'"#,
        ],
    ),
    (12, &[r#"ALTER TABLE activities ADD COLUMN title TEXT"#]),
    (
        13,
        &[
            r#"UPDATE activities SET title = activity_type || ' - ' || date_time"#,
            r#"ALTER TABLE activities ADD COLUMN time TEXT"#,
            r#"UPDATE activities SET time = (SELECT start_time FROM runs WHERE runs.activity_id = activities.id)"#,
        ],
    ),
    (
        14,
        &[r#"CREATE TABLE IF NOT EXISTS settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    age INTEGER CHECK (age >= 0),
    weight REAL CHECK (weight > 0),
    height REAL CHECK (height > 0),
    image TEXT
);"#],
    ),
    (
        15,
        &[r#"CREATE TABLE IF NOT EXISTS shoes (
     id INTEGER PRIMARY KEY AUTOINCREMENT,
     name TEXT NOT NULL,
     distance REAL CHECK (distance >= 0),
     status TEXT,
     image TEXT
);"#],
    ),
    (
        16,
        &[r#"CREATE TABLE IF NOT EXISTS bikes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    weight REAL CHECK (weight > 0),
    distance REAL CHECK (distance >= 0),
    status TEXT,
    image TEXT
);"#],
    ),
    (
        17,
        &[
            r#"ALTER TABLE runs ADD COLUMN shoe_id INTEGER REFERENCES shoes(id) ON DELETE SET NULL"#,
            r#"ALTER TABLE cycling ADD COLUMN bike_id INTEGER REFERENCES bikes(id) ON DELETE SET NULL"#,
            r#"ALTER TABLE activities DROP COLUMN date_time"#,
        ],
    ),
    (
        18,
        &[r#"ALTER TABLE best_performances ADD COLUMN activity_type TEXT"#],
    ),
    (
        19,
        &[
            r#"ALTER TABLE activities ADD COLUMN duration"#,
            r#"UPDATE activities SET duration = (SELECT total_time FROM runs WHERE runs.activity_id = activities.id)"#,
        ],
    ),
    (
        20,
        &[
            r#"ALTER TABLE activities ADD COLUMN duration_new INTEGER"#,
            r#"UPDATE activities SET duration_new = ((substr(total_time, 1, 2) * 3600) + (substr(total_time, 4, 2) * 60) + (substr(total_time, 7, 2))) FROM runs WHERE activities.id = runs.activity_id"#,
            r#"ALTER TABLE activities DROP COLUMN duration"#,
            r#"ALTER TABLE activities RENAME COLUMN duration_new TO duration"#,
        ],
    ),
    (
        22,
        &[
            r#"ALTER TABLE activities ADD COLUMN date_time_new INTEGER"#,
            r#"UPDATE activities SET date_time_new = (SELECT strftime('%s', substr(runs.date, 1, 4) || '-' || substr(runs.date, 6, 2) || '-' || substr(runs.date, 9, 2) || ' ' ||  runs.start_time || ':00') FROM runs WHERE runs.activity_id = activities.id)"#,
            r#"ALTER TABLE activities RENAME COLUMN date_time_new TO date"#,
        ],
    ),
    (
        23,
        &[
            r#"ALTER TABLE runs DROP COLUMN date"#,
            r#"ALTER TABLE runs DROP COLUMN month"#,
            r#"ALTER TABLE runs DROP COLUMN year"#,
            r#"ALTER TABLE runs DROP COLUMN start_time"#,
            r#"ALTER TABLE runs DROP COLUMN total_time"#,
            r#"ALTER TABLE runs DROP COLUMN distance"#,
            r#"ALTER TABLE runs DROP COLUMN activity"#,
        ],
    ),
    (
        24,
        &[
            r#"ALTER TABLE cycling DROP COLUMN date"#,
            r#"ALTER TABLE cycling DROP COLUMN month"#,
            r#"ALTER TABLE cycling DROP COLUMN year"#,
            r#"ALTER TABLE cycling DROP COLUMN start_time"#,
            r#"ALTER TABLE cycling DROP COLUMN total_time"#,
            r#"ALTER TABLE cycling DROP COLUMN distance"#,
            r#"ALTER TABLE cycling DROP COLUMN activity"#,
        ],
    ),
    (
        25,
        &[r#"CREATE TABLE IF NOT EXISTS walking (
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
)"#],
    ),
    (
        26,
        &[
            r#"ALTER TABLE walking ADD COLUMN avg_steps INTEGER"#,
            r#"ALTER TABLE cycling ADD COLUMN total_steps INTEGER"#,
        ],
    ),
    (
        27,
        &[
            r#"ALTER TABLE cycling DROP COLUMN total_steps"#,
            r#"ALTER TABLE walking ADD COLUMN total_steps INTEGER"#,
        ],
    ),
    (
        28,
        &[
            r#"ALTER TABLE activities ADD COLUMN calories INTEGER"#,
            r#"ALTER TABLE settings ADD COLUMN max_heart_rate INTEGER"#,
        ],
    ),
    (
        29,
        &[r#"CREATE TABLE IF NOT EXISTS heart_rate_zones (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    zone INTEGER NOT NULL CHECK (zone BETWEEN 1 AND 5),
    name TEXT NOT NULL,
    lower_bound INTEGER NOT NULL,
    upper_bound INTEGER NOT NULL
)"#],
    ),
    (30, &[r#"ALTER TABLE activities ADD COLUMN file_id TEXT"#]),
    (
        32,
        &[r#"UPDATE activities
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
 AND runs.track_img LIKE '%/images/%_track.png%'"#],
    ),
    (
        33,
        &[r#"UPDATE activities
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
 AND cycling.track_img LIKE '%/images/%_track.png%'"#],
    ),
    (
        34,
        &[r#"UPDATE activities
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
 AND walking.track_img LIKE '%/images/%_track.png%'"#],
    ),
    (
        35,
        &[
            r#"DROP TABLE IF EXISTS run_details"#,
            r#"DROP TABLE IF EXISTS cycling_details"#,
        ],
    ),
    (
        36,
        &[r#"CREATE TABLE IF NOT EXISTS activity_details (
    activity_id TEXT,
    segment_id INTEGER,
    seg_latitude REAL,
    seg_longitude REAL,
    seg_avg_heart_rate REAL,
    seg_avg_power REAL,
    seg_avg_speed REAL,
    seg_avg_pace REAL,
    seg_avg_steps REAL,
    seg_distance REAL,
    seg_time_start TEXT,
    seg_time_end TEXT,
    PRIMARY KEY (activity_id, segment_id)
)"#],
    ),
    (
        37,
        &[r#"CREATE TABLE IF NOT EXISTS weather (
    activity_id TEXT PRIMARY KEY,
    max_temp REAL,
    min_temp REAL,
    precipitation REAL,
    max_wind_speed REAL
)"#],
    ),
    (
        38,
        &[r#"ALTER TABLE activity_details ADD COLUMN seg_elevation_gain REAL"#],
    ),
    (
        39,
        &[r#"ALTER TABLE activities ADD COLUMN elevation_gain TEXT"#],
    ),
    (
        40,
        &[r#"UPDATE activities
SET elevation_gain = (
    SELECT runs.elevation_gain
    FROM runs
    WHERE runs.activity_id = activities.id
)"#],
    ),
    (
        41,
        &[r#"UPDATE activities
SET elevation_gain = (
    SELECT cycling.elevation_gain
    FROM cycling
    WHERE cycling.activity_id = activities.id
)"#],
    ),
    (
        42,
        &[r#"UPDATE activities
SET elevation_gain = (
    SELECT walking.elevation_gain
    FROM walking
    WHERE walking.activity_id = activities.id
)"#],
    ),
    (
        44,
        &[r#"CREATE TABLE activities_new (
    id INTEGER PRIMARY KEY,
    distance REAL,
    activity_type TEXT,
    comment TEXT,
    title TEXT,
    duration INTEGER,
    date INTEGER,
    calories INTEGER,
    file_id TEXT,
    elevation_gain REAL
)"#],
    ),
    (
        45,
        &[
            r#"INSERT INTO activities_new (id, distance, activity_type, comment, title, duration, date, calories, file_id, elevation_gain) SELECT id, distance, activity_type, comment, title, duration, date, calories, file_id, elevation_gain FROM activities"#,
            r#"DROP TABLE activities"#,
            r#"ALTER TABLE activities_new RENAME TO activities"#,
        ],
    ),
    (
        46,
        &[r#"ALTER TABLE weather ADD COLUMN weather_code INTEGER DEFAULT 0"#],
    ),
    (
        47,
        &[r#"CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    name TEXT,
    weight REAL,
    age INTEGER,
    height INTEGER,
    hr_min INTEGER,
    hr_max INTEGER,
    vo2max REAL,
    zone1 INTEGER,
    zone2 INTEGER,
    zone3 INTEGER,
    zone4 INTEGER,
    zone5 INTEGER
)"#],
    ),
    (
        48,
        &[
            r#"ALTER TABLE bikes DROP COLUMN status"#,
            r#"ALTER TABLE shoes DROP COLUMN status"#,
            r#"ALTER TABLE bikes ADD COLUMN status BOOLEAN DEFAULT 1"#,
            r#"ALTER TABLE shoes ADD COLUMN status BOOLEAN DEFAULT 1"#,
        ],
    ),
    (49, &[r#"ALTER TABLE users ADD COLUMN birthday TEXT"#]),
    (
        50,
        &[
            r#"ALTER TABLE activities ADD COLUMN edited BOOLEAN DEFAULT 0"#,
            r#"ALTER TABLE activities ADD COLUMN new BOOLEAN DEFAULT 0"#,
        ],
    ),
    (
        51,
        &[r#"CREATE TABLE IF NOT EXISTS activity_charts (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 activity_id INTEGER NOT NULL, chart_type TEXT NOT NULL,
 file_path TEXT NOT NULL,
 FOREIGN KEY (activity_id) REFERENCES activities(id))"#],
    ),
    (
        52,
        &[
            r#"ALTER TABLE walking ADD COLUMN shoe_id INTEGER REFERENCES shoes(id) ON DELETE SET NULL"#,
        ],
    ),
    (
        53,
        &[r#"ALTER TABLE activities ADD COLUMN type_detail TEXT"#],
    ),
    (
        54,
        &[r#"CREATE TABLE IF NOT EXISTS goals (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    goal_type TEXT NOT NULL,
    activity_types TEXT, -- comma-separated: 'run,walk'
    parameters TEXT NOT NULL -- JSON blob (flexible per goal_type)
)"#],
    ),
    (
        55,
        &[r#"CREATE TABLE IF NOT EXISTS achievements (
    id INTEGER PRIMARY KEY,
    goal_id INTEGER REFERENCES goals(id),
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    result TEXT NOT NULL
)"#],
    ),
    // Rust-only migrations from here on. They only add to the schema, so the Python app
    // (which stops at 55) can still open the database.
    (
        56,
        &[r#"CREATE TABLE IF NOT EXISTS connector_links (
    activity_id INTEGER NOT NULL,
    connector TEXT NOT NULL,
    remote_id TEXT,
    direction TEXT NOT NULL,
    linked_at INTEGER NOT NULL,
    PRIMARY KEY (activity_id, connector)
)"#],
    ),
    // "male" or "female", for comparing VO2max with people of the same age and sex.
    (57, &[r#"ALTER TABLE users ADD COLUMN gender TEXT"#]),
    // Summaries written by a local AI model, cached per activity.
    (
        58,
        &[r#"CREATE TABLE IF NOT EXISTS ai_summaries (
    activity_id INTEGER PRIMARY KEY,
    model TEXT NOT NULL,
    text TEXT NOT NULL,
    created INTEGER NOT NULL
)"#],
    ),
    // One-time corrections of stored values, by name, so each runs once.
    (
        59,
        &[r#"CREATE TABLE IF NOT EXISTS data_fixes (
    name TEXT PRIMARY KEY,
    applied INTEGER NOT NULL
)"#],
    ),
    // Route segments marked on a map, and every pass over them found in the activities.
    (
        60,
        &[
            r#"CREATE TABLE IF NOT EXISTS route_segments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    view TEXT NOT NULL,
    source_activity_id INTEGER,
    created INTEGER NOT NULL,
    distance REAL NOT NULL,
    elevation_gain REAL NOT NULL,
    elevation_loss REAL NOT NULL,
    avg_grade REAL NOT NULL,
    max_grade REAL NOT NULL,
    min_elevation REAL NOT NULL,
    max_elevation REAL NOT NULL,
    points TEXT NOT NULL
)"#,
            r#"CREATE TABLE IF NOT EXISTS segment_efforts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    segment_id INTEGER NOT NULL,
    activity_id INTEGER NOT NULL,
    start_km REAL NOT NULL,
    end_km REAL NOT NULL,
    elapsed REAL NOT NULL,
    avg_heart_rate REAL,
    date INTEGER NOT NULL
)"#,
            "CREATE INDEX IF NOT EXISTS segment_efforts_segment ON segment_efforts (segment_id, elapsed)",
            "CREATE INDEX IF NOT EXISTS segment_efforts_activity ON segment_efforts (activity_id)",
        ],
    ),
    // Goals set by the user: when they were made and whether they are put away.
    (
        61,
        &[
            "ALTER TABLE goals ADD COLUMN archived BOOLEAN NOT NULL DEFAULT 0",
            "ALTER TABLE goals ADD COLUMN created INTEGER",
        ],
    ),
];
