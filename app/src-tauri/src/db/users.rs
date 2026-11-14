//! User profile, heart-rate zones, shoes and bikes, ported from `database/user_settings.py`.
//! The app has a single user: the first row of `users`.

use chrono::{Datelike, Local, NaiveDate};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::value::{bool_or, opt_f64, opt_i64, opt_string};
use crate::error::{AppError, AppResult};

/// Birthday format stored by the Python app.
const BIRTHDAY_FORMAT: &str = "%d.%m.%Y";

/// Sex for the VO2max comparison tables, which only exist for men and women.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Gender {
    Male,
    Female,
}

impl Gender {
    fn as_str(self) -> &'static str {
        match self {
            Self::Male => "male",
            Self::Female => "female",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "male" => Some(Self::Male),
            "female" => Some(Self::Female),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct User {
    #[ts(type = "number")]
    pub id: i64,
    pub name: Option<String>,
    /// kg.
    pub weight: Option<f64>,
    /// cm.
    #[ts(type = "number | null")]
    pub height: Option<i64>,
    /// "DD.MM.YYYY".
    pub birthday: Option<String>,
    #[ts(type = "number | null")]
    pub age: Option<i64>,
    #[ts(type = "number | null")]
    pub hr_min: Option<i64>,
    #[ts(type = "number | null")]
    pub hr_max: Option<i64>,
    pub vo2max: Option<f64>,
    pub gender: Option<Gender>,
    /// Upper bound (bpm) of zones 1–5; `None` until the zones have been saved.
    #[ts(type = "[number, number, number, number, number] | null")]
    pub zones: Option<[i64; 5]>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct UserInput {
    pub name: String,
    pub weight: f64,
    #[ts(type = "number")]
    pub height: i64,
    /// "DD.MM.YYYY".
    pub birthday: String,
    #[ts(type = "number")]
    pub hr_min: i64,
    #[ts(type = "number")]
    pub hr_max: i64,
    #[serde(default)]
    #[ts(optional)]
    pub gender: Option<Gender>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ZonesInput {
    pub vo2max: f64,
    #[ts(type = "number")]
    pub hr_min: i64,
    /// Upper bound (bpm) of zones 1–5, ascending.
    #[ts(type = "[number, number, number, number, number]")]
    pub zones: [i64; 5],
}

/// Age in whole years on `today` for a "DD.MM.YYYY" birthday.
pub fn age_on(birthday: &str, today: NaiveDate) -> Option<i64> {
    let born = NaiveDate::parse_from_str(birthday.trim(), BIRTHDAY_FORMAT).ok()?;
    let mut age = today.year() - born.year();
    if (today.month(), today.day()) < (born.month(), born.day()) {
        age -= 1;
    }
    (age >= 0).then_some(i64::from(age))
}

/// Suggested zone bounds, as `window_user_settings.calculate_heart_rate_zones` computes them:
/// 60/70/80/90/100 % of `(hr_max or 220) - age`.
pub fn suggested_zones(hr_max: Option<i64>, age: i64) -> [i64; 5] {
    let max = (hr_max.unwrap_or(220) - age) as f64;
    [0.6, 0.7, 0.8, 0.9, 1.0].map(|share| (max * share) as i64)
}

pub fn get(conn: &Connection) -> AppResult<Option<User>> {
    Ok(conn
        .query_row("SELECT * FROM users ORDER BY id LIMIT 1", [], |row| {
            let zones = [
                opt_i64(row, "zone1")?,
                opt_i64(row, "zone2")?,
                opt_i64(row, "zone3")?,
                opt_i64(row, "zone4")?,
                opt_i64(row, "zone5")?,
            ];
            Ok(User {
                id: row.get("id")?,
                name: opt_string(row, "name")?,
                weight: opt_f64(row, "weight")?,
                height: opt_i64(row, "height")?,
                birthday: opt_string(row, "birthday")?,
                age: opt_i64(row, "age")?,
                hr_min: opt_i64(row, "hr_min")?,
                hr_max: opt_i64(row, "hr_max")?,
                vo2max: opt_f64(row, "vo2max")?,
                gender: opt_string(row, "gender")?
                    .as_deref()
                    .and_then(Gender::parse),
                zones: zones
                    .iter()
                    .all(Option::is_some)
                    .then(|| zones.map(Option::unwrap_or_default)),
            })
        })
        .optional()?)
}

/// Creates the user or updates the existing one; the age is derived from the birthday.
pub fn save(conn: &Connection, input: &UserInput) -> AppResult<User> {
    let age = age_on(&input.birthday, Local::now().date_naive()).ok_or_else(|| {
        AppError::Invalid(format!("birthday must be DD.MM.YYYY: {}", input.birthday))
    })?;
    if input.weight <= 0.0 || input.height <= 0 {
        return Err(AppError::Invalid(
            "weight and height must be positive".into(),
        ));
    }
    if input.hr_min >= input.hr_max {
        return Err(AppError::Invalid(
            "resting heart rate must be below maximum heart rate".into(),
        ));
    }
    let gender = input.gender.map(Gender::as_str);
    match get(conn)? {
        Some(user) => conn.execute(
            "UPDATE users SET name = ?1, weight = ?2, age = ?3, height = ?4, hr_min = ?5,
                hr_max = ?6, birthday = ?7, gender = ?8 WHERE id = ?9",
            params![
                input.name,
                input.weight,
                age,
                input.height,
                input.hr_min,
                input.hr_max,
                input.birthday,
                gender,
                user.id
            ],
        )?,
        None => conn.execute(
            "INSERT INTO users (name, weight, age, height, hr_min, hr_max, birthday, gender)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                input.name,
                input.weight,
                age,
                input.height,
                input.hr_min,
                input.hr_max,
                input.birthday,
                gender
            ],
        )?,
    };
    get(conn)?.ok_or_else(|| AppError::NotFound("user".into()))
}

pub fn save_zones(conn: &Connection, input: &ZonesInput) -> AppResult<User> {
    let user = get(conn)?.ok_or_else(|| AppError::NotFound("user".into()))?;
    if !input.zones.windows(2).all(|pair| pair[0] < pair[1]) {
        return Err(AppError::Invalid("zone bounds must be ascending".into()));
    }
    let [z1, z2, z3, z4, z5] = input.zones;
    conn.execute(
        "UPDATE users SET vo2max = ?1, hr_min = ?2, zone1 = ?3, zone2 = ?4, zone3 = ?5,
            zone4 = ?6, zone5 = ?7 WHERE id = ?8",
        params![input.vo2max, input.hr_min, z1, z2, z3, z4, z5, user.id],
    )?;
    get(conn)?.ok_or_else(|| AppError::NotFound("user".into()))
}

/// Shoes and bikes share one shape; `weight` is only set for bikes.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Equipment {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    /// Total km of all activities using it.
    pub distance: f64,
    /// kg, bikes only.
    pub weight: Option<f64>,
    pub in_use: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum EquipmentKind {
    Shoe,
    Bike,
}

impl EquipmentKind {
    fn table(self) -> &'static str {
        match self {
            Self::Shoe => "shoes",
            Self::Bike => "bikes",
        }
    }

    /// Per-type tables (and column) that reference this kind.
    fn references(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Shoe => &[("runs", "shoe_id"), ("walking", "shoe_id")],
            Self::Bike => &[("cycling", "bike_id")],
        }
    }
}

pub fn list_equipment(conn: &Connection, kind: EquipmentKind) -> AppResult<Vec<Equipment>> {
    let weight = match kind {
        EquipmentKind::Shoe => "NULL",
        EquipmentKind::Bike => "weight",
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT id, name, distance, {weight} AS weight, status FROM {} ORDER BY id",
        kind.table()
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(Equipment {
            id: row.get("id")?,
            name: opt_string(row, "name")?.unwrap_or_default(),
            distance: opt_f64(row, "distance")?.unwrap_or(0.0),
            weight: opt_f64(row, "weight")?,
            in_use: bool_or(row, "status", true)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Adds a shoe or bike with zero distance. (The Python `insert_bike` swapped weight and status.)
pub fn add_equipment(
    conn: &Connection,
    kind: EquipmentKind,
    name: &str,
    weight: Option<f64>,
    in_use: bool,
) -> AppResult<i64> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::Invalid("name must not be empty".into()));
    }
    match kind {
        EquipmentKind::Shoe => conn.execute(
            "INSERT INTO shoes (name, distance, status) VALUES (?1, 0, ?2)",
            params![name, in_use],
        )?,
        EquipmentKind::Bike => conn.execute(
            "INSERT INTO bikes (name, weight, distance, status) VALUES (?1, ?2, 0, ?3)",
            params![name, weight.filter(|w| *w > 0.0), in_use],
        )?,
    };
    Ok(conn.last_insert_rowid())
}

pub fn set_equipment_in_use(
    conn: &Connection,
    kind: EquipmentKind,
    id: i64,
    in_use: bool,
) -> AppResult<()> {
    let changed = conn.execute(
        &format!("UPDATE {} SET status = ?1 WHERE id = ?2", kind.table()),
        params![in_use, id],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound(format!("{} {id}", kind.table())));
    }
    Ok(())
}

/// Deletes a shoe or bike and unassigns it from activities. Foreign keys are off in the
/// shared database, so `ON DELETE SET NULL` never ran in the Python app.
pub fn delete_equipment(conn: &mut Connection, kind: EquipmentKind, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    for (table, column) in kind.references() {
        tx.execute(
            &format!("UPDATE {table} SET {column} = NULL WHERE {column} = ?1"),
            [id],
        )?;
    }
    tx.execute(&format!("DELETE FROM {} WHERE id = ?1", kind.table()), [id])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    fn input() -> UserInput {
        UserInput {
            name: "Axel".into(),
            weight: 68.0,
            height: 168,
            birthday: "31.10.1968".into(),
            hr_min: 50,
            hr_max: 185,
            gender: Some(Gender::Male),
        }
    }

    #[test]
    fn age_counts_completed_years() {
        let day = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        assert_eq!(age_on("31.10.1968", day(2026, 10, 30)), Some(57));
        assert_eq!(age_on("31.10.1968", day(2026, 10, 31)), Some(58));
        assert_eq!(age_on("1968-10-31", day(2026, 1, 1)), None);
    }

    #[test]
    fn suggested_zones_match_python() {
        // hr_max 220 and age 56 give the zone5 bound stored in the real database (164).
        assert_eq!(suggested_zones(Some(220), 56), [98, 114, 131, 147, 164]);
    }

    #[test]
    fn saves_user_then_zones() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        assert!(get(&conn).unwrap().is_none());

        let user = save(&conn, &input()).unwrap();
        assert!(user.zones.is_none());
        assert_eq!(user.gender, Some(Gender::Male));
        let again = save(
            &conn,
            &UserInput {
                weight: 70.0,
                ..input()
            },
        )
        .unwrap();
        assert_eq!(again.id, user.id);
        assert_eq!(again.weight, Some(70.0));

        let user = save_zones(
            &conn,
            &ZonesInput {
                vo2max: 48.0,
                hr_min: 50,
                zones: [110, 130, 150, 165, 185],
            },
        )
        .unwrap();
        assert_eq!(user.zones, Some([110, 130, 150, 165, 185]));
    }

    #[test]
    fn rejects_invalid_input() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        let bad = UserInput {
            birthday: "1968".into(),
            ..input()
        };
        assert!(matches!(save(&conn, &bad), Err(AppError::Invalid(_))));
        save(&conn, &input()).unwrap();
        let zones = ZonesInput {
            vo2max: 0.0,
            hr_min: 50,
            zones: [150, 140, 160, 170, 180],
        };
        assert!(matches!(
            save_zones(&conn, &zones),
            Err(AppError::Invalid(_))
        ));
    }

    #[test]
    fn equipment_lifecycle() {
        let db = Db::open_in_memory().unwrap();
        let mut conn = db.conn().unwrap();
        let bike = add_equipment(&conn, EquipmentKind::Bike, "Gravel", Some(10.5), true).unwrap();
        let bikes = list_equipment(&conn, EquipmentKind::Bike).unwrap();
        assert_eq!(bikes[0].weight, Some(10.5));
        assert!(bikes[0].in_use);

        set_equipment_in_use(&conn, EquipmentKind::Bike, bike, false).unwrap();
        assert!(!list_equipment(&conn, EquipmentKind::Bike).unwrap()[0].in_use);

        insert_activity(&conn, 1, "Biking", 0, 20.0, "Ride");
        conn.execute("UPDATE cycling SET bike_id = ?1", [bike])
            .unwrap();
        delete_equipment(&mut conn, EquipmentKind::Bike, bike).unwrap();
        assert!(
            list_equipment(&conn, EquipmentKind::Bike)
                .unwrap()
                .is_empty()
        );
        let assigned: Option<i64> = conn
            .query_row(
                "SELECT bike_id FROM cycling WHERE activity_id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(assigned, None);
    }
}
