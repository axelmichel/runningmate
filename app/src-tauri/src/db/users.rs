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

/// How the heart-rate zones are set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum ZoneMethod {
    /// The user's own upper bounds (`zone1`–`zone5`).
    #[default]
    Custom,
    /// Shares of the maximum heart rate.
    PercentMax,
    /// Shares of the heart-rate reserve (max − rest) on top of the resting heart rate.
    Karvonen,
}

impl ZoneMethod {
    fn as_str(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::PercentMax => "percentMax",
            Self::Karvonen => "karvonen",
        }
    }

    fn parse(value: &str) -> Self {
        match value {
            "percentMax" => Self::PercentMax,
            "karvonen" => Self::Karvonen,
            _ => Self::Custom,
        }
    }
}

/// Effective heart-rate zones: where zone 1 starts and the upper bound of each zone (bpm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ZoneBounds {
    #[ts(type = "number")]
    pub floor: i64,
    /// Upper bound of zones 1–5, ascending.
    #[ts(type = "[number, number, number, number, number]")]
    pub upper: [i64; 5],
}

/// Upper bounds of zones 1–5 in percent for the derived methods; zone 1 starts at `FLOOR_PERCENT`.
const ZONE_PERCENTS: [i64; 5] = [60, 70, 80, 90, 100];
const FLOOR_PERCENT: i64 = 50;

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
    pub zone_method: ZoneMethod,
    /// The user's own upper bounds (bpm) of zones 1–5; `None` until they have been saved.
    /// Kept when another method is chosen, so switching back restores them.
    #[ts(type = "[number, number, number, number, number] | null")]
    pub custom_zones: Option<[i64; 5]>,
    /// The zones in effect for `zone_method` (see [`resolve_zones`]); `None` while the method
    /// lacks what it needs: saved custom bounds, a maximum or (Karvonen) a resting heart rate.
    pub zones: Option<ZoneBounds>,
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
    pub method: ZoneMethod,
    /// Upper bound (bpm) of zones 1–5, ascending; required for `custom`, kept as they are when
    /// missing for the other methods.
    #[serde(default)]
    #[ts(optional, type = "[number, number, number, number, number] | null")]
    pub zones: Option<[i64; 5]>,
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

/// Zones at `ZONE_PERCENTS` of the range from `base` up to `max`, rounded down.
fn share_of(base: i64, max: i64) -> ZoneBounds {
    let at = |percent: i64| base + (max - base) * percent / 100;
    ZoneBounds {
        floor: at(FLOOR_PERCENT),
        upper: ZONE_PERCENTS.map(at),
    }
}

/// The zones in effect, the one place every consumer gets them from (via [`get`] or [`zones`]):
/// - `Custom`: the saved bounds; zone 1 starts at half of the zone 5 bound, as in Python.
/// - `PercentMax`: 60/70/80/90/100 % of the maximum heart rate, zone 1 from 50 %.
/// - `Karvonen`: rest + p × (max − rest) for the same shares p.
///
/// `None` when the method lacks its inputs; heart rates of 0 count as missing.
pub fn resolve_zones(
    method: ZoneMethod,
    custom: Option<[i64; 5]>,
    hr_min: Option<i64>,
    hr_max: Option<i64>,
) -> Option<ZoneBounds> {
    let max = hr_max.filter(|m| *m > 0);
    match method {
        ZoneMethod::Custom => custom.map(|upper| ZoneBounds {
            floor: upper[4] / 2,
            upper,
        }),
        ZoneMethod::PercentMax => max.map(|max| share_of(0, max)),
        ZoneMethod::Karvonen => {
            let rest = hr_min.filter(|r| *r > 0)?;
            max.filter(|max| rest < *max).map(|max| share_of(rest, max))
        }
    }
}

/// The user's zones in effect; `None` without a user or while the method lacks its inputs.
pub fn zones(conn: &Connection) -> AppResult<Option<ZoneBounds>> {
    Ok(get(conn)?.and_then(|u| u.zones))
}

/// Suggested custom bounds: the `PercentMax` zones of the profile's maximum heart rate or,
/// without one, of `220 - age`. (The Python app subtracted the age from the profile value too,
/// so a measured maximum of 185 at 56 gave a zone 5 of 129.)
pub fn suggested_zones(hr_max: Option<i64>, age: i64) -> [i64; 5] {
    let max = hr_max.filter(|m| *m > 0).unwrap_or(220 - age);
    share_of(0, max).upper
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
            let custom_zones = zones
                .iter()
                .all(Option::is_some)
                .then(|| zones.map(Option::unwrap_or_default));
            let zone_method =
                ZoneMethod::parse(&opt_string(row, "zone_method")?.unwrap_or_default());
            let hr_min = opt_i64(row, "hr_min")?;
            let hr_max = opt_i64(row, "hr_max")?;
            Ok(User {
                id: row.get("id")?,
                name: opt_string(row, "name")?,
                weight: opt_f64(row, "weight")?,
                height: opt_i64(row, "height")?,
                birthday: opt_string(row, "birthday")?,
                age: opt_i64(row, "age")?,
                hr_min,
                hr_max,
                vo2max: opt_f64(row, "vo2max")?,
                gender: opt_string(row, "gender")?
                    .as_deref()
                    .and_then(Gender::parse),
                zone_method,
                custom_zones,
                zones: resolve_zones(zone_method, custom_zones, hr_min, hr_max),
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

/// Saves VO2max and the zone method; custom bounds are stored when given (and required for
/// `custom`). The derived methods need a maximum and, for Karvonen, a resting heart rate.
pub fn save_zones(conn: &Connection, input: &ZonesInput) -> AppResult<User> {
    let user = get(conn)?.ok_or_else(|| AppError::NotFound("user".into()))?;
    if let Some(bounds) = input.zones
        && !bounds.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(AppError::Invalid("zone bounds must be ascending".into()));
    }
    let custom = input.zones.or(user.custom_zones);
    if resolve_zones(input.method, custom, Some(input.hr_min), user.hr_max).is_none() {
        return Err(AppError::Invalid(
            match input.method {
                ZoneMethod::Custom => "custom zones need five bounds",
                ZoneMethod::PercentMax => {
                    "zones from the maximum heart rate need one in the profile"
                }
                ZoneMethod::Karvonen => {
                    "Karvonen zones need a resting and a maximum heart rate in the profile"
                }
            }
            .into(),
        ));
    }
    conn.execute(
        "UPDATE users SET vo2max = ?1, hr_min = ?2, zone_method = ?3 WHERE id = ?4",
        params![input.vo2max, input.hr_min, input.method.as_str(), user.id],
    )?;
    if let Some([z1, z2, z3, z4, z5]) = input.zones {
        conn.execute(
            "UPDATE users SET zone1 = ?1, zone2 = ?2, zone3 = ?3, zone4 = ?4, zone5 = ?5
             WHERE id = ?6",
            params![z1, z2, z3, z4, z5, user.id],
        )?;
    }
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
    fn suggested_zones_use_the_maximum_heart_rate() {
        assert_eq!(suggested_zones(Some(185), 56), [111, 129, 148, 166, 185]);
        // Without one, 220 - age; age 56 gives the zone5 bound stored in the real database.
        assert_eq!(suggested_zones(None, 56), [98, 114, 131, 147, 164]);
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
                method: ZoneMethod::Custom,
                zones: Some([110, 130, 150, 165, 185]),
            },
        )
        .unwrap();
        assert_eq!(user.custom_zones, Some([110, 130, 150, 165, 185]));
        assert_eq!(
            user.zones,
            Some(ZoneBounds {
                floor: 92,
                upper: [110, 130, 150, 165, 185]
            })
        );

        // A derived method keeps the custom bounds for switching back; the profile drives it.
        let user = save_zones(
            &conn,
            &ZonesInput {
                vo2max: 48.0,
                hr_min: 50,
                method: ZoneMethod::Karvonen,
                zones: None,
            },
        )
        .unwrap();
        assert_eq!(user.zone_method, ZoneMethod::Karvonen);
        assert_eq!(user.custom_zones, Some([110, 130, 150, 165, 185]));
        assert_eq!(user.zones.unwrap().upper, [131, 144, 158, 171, 185]);
        save(
            &conn,
            &UserInput {
                hr_min: 60,
                ..input()
            },
        )
        .unwrap();
        assert_eq!(
            zones(&conn).unwrap().unwrap().upper,
            [135, 147, 160, 172, 185]
        );
    }

    #[test]
    fn resolves_zones_for_each_method() {
        let custom = Some([110, 130, 150, 165, 185]);
        let resolve = |method| resolve_zones(method, custom, Some(50), Some(185));
        assert_eq!(
            resolve(ZoneMethod::Custom),
            Some(ZoneBounds {
                floor: 92,
                upper: [110, 130, 150, 165, 185]
            })
        );
        // 185 × 50/60/70/80/90/100 %, rounded down as the suggestion is.
        assert_eq!(
            resolve(ZoneMethod::PercentMax),
            Some(ZoneBounds {
                floor: 92,
                upper: [111, 129, 148, 166, 185]
            })
        );
        // 50 + p × 135.
        assert_eq!(
            resolve(ZoneMethod::Karvonen),
            Some(ZoneBounds {
                floor: 117,
                upper: [131, 144, 158, 171, 185]
            })
        );
    }

    #[test]
    fn zones_need_their_inputs() {
        let custom = Some([110, 130, 150, 165, 185]);
        assert_eq!(
            resolve_zones(ZoneMethod::Custom, None, Some(50), Some(185)),
            None
        );
        assert_eq!(
            resolve_zones(ZoneMethod::PercentMax, custom, Some(50), None),
            None
        );
        assert_eq!(
            resolve_zones(ZoneMethod::PercentMax, custom, None, Some(0)),
            None
        );
        // Percent of max doesn't need a resting heart rate; Karvonen does.
        assert!(resolve_zones(ZoneMethod::PercentMax, None, None, Some(185)).is_some());
        assert_eq!(
            resolve_zones(ZoneMethod::Karvonen, custom, None, Some(185)),
            None
        );
        assert_eq!(
            resolve_zones(ZoneMethod::Karvonen, custom, Some(0), Some(185)),
            None
        );
        assert_eq!(
            resolve_zones(ZoneMethod::Karvonen, custom, Some(50), None),
            None
        );
        assert_eq!(
            resolve_zones(ZoneMethod::Karvonen, custom, Some(190), Some(185)),
            None
        );
    }

    #[test]
    fn existing_users_keep_custom_zones() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        conn.execute(
            "INSERT INTO users (name, hr_min, hr_max, zone1, zone2, zone3, zone4, zone5)
             VALUES ('Axel', 50, 185, 98, 114, 131, 147, 164)",
            [],
        )
        .unwrap();
        let user = get(&conn).unwrap().unwrap();
        assert_eq!(user.zone_method, ZoneMethod::Custom);
        assert_eq!(user.zones.unwrap().upper, [98, 114, 131, 147, 164]);
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
            method: ZoneMethod::Custom,
            zones: Some([150, 140, 160, 170, 180]),
        };
        assert!(matches!(
            save_zones(&conn, &zones),
            Err(AppError::Invalid(_))
        ));
        // Custom without bounds (none saved yet), and Karvonen without a resting heart rate.
        for (method, hr_min) in [(ZoneMethod::Custom, 50), (ZoneMethod::Karvonen, 0)] {
            let zones = ZonesInput {
                vo2max: 0.0,
                hr_min,
                method,
                zones: None,
            };
            assert!(matches!(
                save_zones(&conn, &zones),
                Err(AppError::Invalid(_))
            ));
        }
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
