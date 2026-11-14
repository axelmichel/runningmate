//! Garmin credentials in the system keychain (service `garmin`), shared with the Python app,
//! which stored `username` and `password` there via the `keyring` package.

use keyring::Entry;

use super::client::OAuth1Token;
use crate::error::{AppError, AppResult};

const SERVICE: &str = "garmin";
const TOKEN: &str = "oauth1_token";
const USERNAME: &str = "username";
const PASSWORD: &str = "password";

fn entry(name: &str) -> AppResult<Entry> {
    Entry::new(SERVICE, name).map_err(keychain)
}

fn keychain(error: keyring::Error) -> AppError {
    AppError::Invalid(format!("keychain: {error}"))
}

fn read(name: &str) -> AppResult<Option<String>> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(keychain(e)),
    }
}

fn remove(name: &str) -> AppResult<()> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(keychain(e)),
    }
}

pub fn token() -> AppResult<Option<OAuth1Token>> {
    Ok(read(TOKEN)?.and_then(|json| serde_json::from_str(&json).ok()))
}

/// Stores the token. The password is not stored: the token is enough for later syncs.
pub fn save_token(token: &OAuth1Token) -> AppResult<()> {
    entry(TOKEN)?
        .set_password(&serde_json::to_string(token)?)
        .map_err(keychain)
}

/// Username and password saved by the Python app, if any.
pub fn legacy_login() -> AppResult<Option<(String, String)>> {
    Ok(match (read(USERNAME)?, read(PASSWORD)?) {
        (Some(user), Some(password)) => Some((user, password)),
        _ => None,
    })
}

/// Forgets the token and any password the Python app saved.
pub fn forget() -> AppResult<()> {
    remove(TOKEN)?;
    remove(USERNAME)?;
    remove(PASSWORD)
}
