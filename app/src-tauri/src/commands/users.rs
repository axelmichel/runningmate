use tauri::State;

use crate::{
    db::{
        Db,
        users::{
            self, Equipment, EquipmentKind, User, UserInput, ZoneBounds, ZoneMethod, ZonesInput,
        },
    },
    error::{AppError, AppResult},
};

#[tauri::command(async)]
pub fn get_user(db: State<'_, Db>) -> AppResult<Option<User>> {
    users::get(&*db.conn()?)
}

#[tauri::command(async)]
pub fn save_user(db: State<'_, Db>, input: UserInput) -> AppResult<User> {
    users::save(&*db.conn()?, &input)
}

#[tauri::command(async)]
pub fn save_heart_rate_zones(db: State<'_, Db>, input: ZonesInput) -> AppResult<User> {
    users::save_zones(&*db.conn()?, &input)
}

/// Zone bounds to pre-fill the zones form for a user who hasn't saved any yet.
#[tauri::command(async)]
pub fn suggested_heart_rate_zones(db: State<'_, Db>) -> AppResult<[i64; 5]> {
    let user = users::get(&*db.conn()?)?.ok_or_else(|| AppError::NotFound("user".into()))?;
    let today = chrono::Local::now().date_naive();
    let age = user
        .birthday
        .as_deref()
        .and_then(|birthday| users::age_on(birthday, today))
        .or(user.age)
        .unwrap_or(0);
    Ok(users::suggested_zones(user.hr_max, age))
}

/// The zones `method` would give with the saved profile and custom bounds, for the zones form
/// to show before it is saved; `None` while the method lacks its inputs.
#[tauri::command(async)]
pub fn preview_heart_rate_zones(
    db: State<'_, Db>,
    method: ZoneMethod,
) -> AppResult<Option<ZoneBounds>> {
    let user = users::get(&*db.conn()?)?.ok_or_else(|| AppError::NotFound("user".into()))?;
    Ok(users::resolve_zones(
        method,
        user.custom_zones,
        user.hr_min,
        user.hr_max,
    ))
}

#[tauri::command(async)]
pub fn list_equipment(db: State<'_, Db>, kind: EquipmentKind) -> AppResult<Vec<Equipment>> {
    users::list_equipment(&*db.conn()?, kind)
}

#[tauri::command(async)]
pub fn add_equipment(
    db: State<'_, Db>,
    kind: EquipmentKind,
    name: String,
    weight: Option<f64>,
    in_use: bool,
) -> AppResult<Vec<Equipment>> {
    let conn = db.conn()?;
    users::add_equipment(&conn, kind, &name, weight, in_use)?;
    users::list_equipment(&conn, kind)
}

#[tauri::command(async)]
pub fn set_equipment_in_use(
    db: State<'_, Db>,
    kind: EquipmentKind,
    id: i64,
    in_use: bool,
) -> AppResult<Vec<Equipment>> {
    let conn = db.conn()?;
    users::set_equipment_in_use(&conn, kind, id, in_use)?;
    users::list_equipment(&conn, kind)
}

#[tauri::command(async)]
pub fn delete_equipment(
    db: State<'_, Db>,
    kind: EquipmentKind,
    id: i64,
) -> AppResult<Vec<Equipment>> {
    let mut conn = db.conn()?;
    users::delete_equipment(&mut conn, kind, id)?;
    users::list_equipment(&conn, kind)
}
