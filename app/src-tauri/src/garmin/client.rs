//! Garmin Connect sign-in and API access, ported from `garth` (used by `garminconnect`).
//!
//! Sign-in uses Garmin's SSO web form, then trades the resulting ticket for an OAuth1 token
//! (valid for about a year) and that for a short-lived OAuth2 bearer token for API calls.

use std::{sync::LazyLock, time::Duration};

use regex::Regex;
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use url::Url;

use super::oauth1::{self, Credentials};
use crate::error::{AppError, AppResult};

const SSO: &str = "https://sso.garmin.com/sso";
const SSO_EMBED: &str = "https://sso.garmin.com/sso/embed";
const OAUTH_BASE: &str = "https://connectapi.garmin.com/oauth-service/oauth/";
const API_BASE: &str = "https://connectapi.garmin.com";
/// Garmin's mobile-app OAuth consumer, published by garth.
const CONSUMER_URL: &str = "https://thegarth.s3.amazonaws.com/oauth_consumer.json";
const API_USER_AGENT: &str = "GCM-iOS-5.7.2.1";
const OAUTH_USER_AGENT: &str = "com.garmin.android.apps.connectmobile";

static CSRF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"name="_csrf"\s+value="(.+?)""#).unwrap());
static TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<title>(.+?)</title>").unwrap());
static TICKET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"embed\?ticket=([^"]+)""#).unwrap());

fn garmin_error(message: impl Into<String>) -> AppError {
    AppError::Invalid(format!("Garmin Connect: {}", message.into()))
}

fn http(error: reqwest::Error) -> AppError {
    garmin_error(error.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct Consumer {
    consumer_key: String,
    consumer_secret: String,
}

/// Long-lived token; stored in the keychain so later syncs need neither password nor MFA.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OAuth1Token {
    pub oauth_token: String,
    pub oauth_token_secret: String,
    #[serde(default)]
    pub mfa_token: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct OAuth2Token {
    access_token: String,
}

/// Outcome of submitting username and password.
pub enum SignIn {
    Done(OAuth1Token),
    /// The account uses two-step verification; continue with [`PendingSignIn::submit_code`].
    NeedsCode(PendingSignIn),
}

/// A sign-in waiting for its MFA code; keeps the cookies of the SSO session.
pub struct PendingSignIn {
    client: Client,
    csrf: String,
    referer: String,
}

fn signin_params() -> Vec<(&'static str, &'static str)> {
    vec![
        ("id", "gauth-widget"),
        ("embedWidget", "true"),
        ("gauthHost", SSO_EMBED),
        ("service", SSO_EMBED),
        ("source", SSO_EMBED),
        ("redirectAfterAccountLoginUrl", SSO_EMBED),
        ("redirectAfterAccountCreationUrl", SSO_EMBED),
    ]
}

fn capture(re: &Regex, html: &str, what: &str) -> AppResult<String> {
    re.captures(html)
        .map(|c| c[1].to_string())
        .ok_or_else(|| garmin_error(format!("unexpected sign-in page ({what} missing)")))
}

fn sso_client() -> AppResult<Client> {
    Client::builder()
        .cookie_store(true)
        .user_agent(API_USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(http)
}

async fn consumer(client: &Client) -> AppResult<Consumer> {
    client
        .get(CONSUMER_URL)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .json()
        .await
        .map_err(http)
}

/// Signs in with username and password.
pub async fn sign_in(username: &str, password: &str) -> AppResult<SignIn> {
    let client = sso_client()?;
    // Sets the SSO cookies.
    let embed = client
        .get(SSO_EMBED)
        .query(&[
            ("id", "gauth-widget"),
            ("embedWidget", "true"),
            ("gauthHost", SSO),
        ])
        .send()
        .await
        .map_err(http)?;
    let embed_url = embed.url().to_string();

    let form = client
        .get(format!("{SSO}/signin"))
        .query(&signin_params())
        .header(header::REFERER, embed_url)
        .send()
        .await
        .map_err(http)?;
    let signin_url = form.url().to_string();
    let csrf = capture(&CSRF, &form.text().await.map_err(http)?, "CSRF token")?;

    let response = client
        .post(format!("{SSO}/signin"))
        .query(&signin_params())
        .header(header::REFERER, &signin_url)
        .form(&[
            ("username", username),
            ("password", password),
            ("embed", "true"),
            ("_csrf", &csrf),
        ])
        .send()
        .await
        .map_err(http)?;
    let referer = response.url().to_string();
    let html = response.text().await.map_err(http)?;
    let title = capture(&TITLE, &html, "title")?;

    if title.contains("MFA") {
        let csrf = capture(&CSRF, &html, "CSRF token").unwrap_or(csrf);
        return Ok(SignIn::NeedsCode(PendingSignIn {
            client,
            csrf,
            referer,
        }));
    }
    finish(&client, &title, &html).await.map(SignIn::Done)
}

impl PendingSignIn {
    /// Completes a sign-in with the code from the authenticator app, e-mail or SMS.
    pub async fn submit_code(self, code: &str) -> AppResult<OAuth1Token> {
        let response = self
            .client
            .post(format!("{SSO}/verifyMFA/loginEnterMfaCode"))
            .query(&signin_params())
            .header(header::REFERER, &self.referer)
            .form(&[
                ("mfa-code", code.trim()),
                ("embed", "true"),
                ("_csrf", &self.csrf),
                ("fromPage", "setupEnterMfaCode"),
            ])
            .send()
            .await
            .map_err(http)?;
        let html = response.text().await.map_err(http)?;
        let title = capture(&TITLE, &html, "title")?;
        finish(&self.client, &title, &html).await
    }
}

async fn finish(client: &Client, title: &str, html: &str) -> AppResult<OAuth1Token> {
    if title != "Success" {
        return Err(garmin_error(
            "sign-in failed; check e-mail address, password or code",
        ));
    }
    let ticket = capture(&TICKET, html, "ticket")?;
    oauth1_token(client, &ticket).await
}

fn nonce() -> String {
    format!(
        "{:016x}{:016x}",
        rand::random::<u64>(),
        rand::random::<u64>()
    )
}

fn now() -> u64 {
    chrono::Utc::now().timestamp().max(0) as u64
}

/// Trades an SSO ticket for an OAuth1 token.
async fn oauth1_token(client: &Client, ticket: &str) -> AppResult<OAuth1Token> {
    let consumer = consumer(client).await?;
    let mut url = Url::parse(&format!("{OAUTH_BASE}preauthorized")).expect("valid URL");
    url.query_pairs_mut()
        .append_pair("ticket", ticket)
        .append_pair("login-url", SSO_EMBED)
        .append_pair("accepts-mfa-tokens", "true");
    let auth = oauth1::authorization(
        "GET",
        &url,
        &[],
        &Credentials {
            consumer_key: &consumer.consumer_key,
            consumer_secret: &consumer.consumer_secret,
            token: None,
        },
        &nonce(),
        now(),
    );
    let body = client
        .get(url)
        .header(header::AUTHORIZATION, auth)
        .header(header::USER_AGENT, OAUTH_USER_AGENT)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .text()
        .await
        .map_err(http)?;
    let fields: std::collections::HashMap<String, String> =
        url::form_urlencoded::parse(body.as_bytes())
            .into_owned()
            .collect();
    Ok(OAuth1Token {
        oauth_token: fields
            .get("oauth_token")
            .cloned()
            .ok_or_else(|| garmin_error("no OAuth token in response"))?,
        oauth_token_secret: fields
            .get("oauth_token_secret")
            .cloned()
            .ok_or_else(|| garmin_error("no OAuth token secret in response"))?,
        mfa_token: fields.get("mfa_token").cloned(),
    })
}

/// An authenticated API session.
pub struct Api {
    client: Client,
    bearer: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivitySummary {
    pub activity_id: u64,
    /// "YYYY-MM-DD HH:MM:SS" in the activity's local time.
    pub start_time_local: String,
    #[serde(default)]
    pub activity_name: Option<String>,
}

impl ActivitySummary {
    /// "YYYY-MM-DD" of the start.
    pub fn date(&self) -> &str {
        self.start_time_local
            .get(..10)
            .unwrap_or(&self.start_time_local)
    }
}

impl Api {
    /// Exchanges the stored OAuth1 token for an OAuth2 bearer token.
    pub async fn connect(token: &OAuth1Token) -> AppResult<Self> {
        let client = sso_client()?;
        let consumer = consumer(&client).await?;
        let url = Url::parse(&format!("{OAUTH_BASE}exchange/user/2.0")).expect("valid URL");
        let body: Vec<(String, String)> = token
            .mfa_token
            .iter()
            .map(|m| ("mfa_token".to_string(), m.clone()))
            .collect();
        let auth = oauth1::authorization(
            "POST",
            &url,
            &body,
            &Credentials {
                consumer_key: &consumer.consumer_key,
                consumer_secret: &consumer.consumer_secret,
                token: Some((&token.oauth_token, &token.oauth_token_secret)),
            },
            &nonce(),
            now(),
        );
        let response = client
            .post(url)
            .header(header::AUTHORIZATION, auth)
            .header(header::USER_AGENT, OAUTH_USER_AGENT)
            .form(&body)
            .send()
            .await
            .map_err(http)?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(AppError::GarminSignInRequired);
        }
        let oauth2: OAuth2Token = response
            .error_for_status()
            .map_err(http)?
            .json()
            .await
            .map_err(http)?;
        Ok(Self {
            client,
            bearer: format!("Bearer {}", oauth2.access_token),
        })
    }

    /// Activities started between two dates ("YYYY-MM-DD", inclusive), oldest first.
    pub async fn activities(&self, start: &str, end: &str) -> AppResult<Vec<ActivitySummary>> {
        const PAGE: usize = 100;
        let mut all = Vec::new();
        loop {
            let offset = all.len().to_string();
            let page: Vec<ActivitySummary> = self
                .client
                .get(format!(
                    "{API_BASE}/activitylist-service/activities/search/activities"
                ))
                .query(&[
                    ("startDate", start),
                    ("endDate", end),
                    ("start", offset.as_str()),
                    ("limit", &PAGE.to_string()),
                ])
                .header(header::AUTHORIZATION, &self.bearer)
                .send()
                .await
                .and_then(|r| r.error_for_status())
                .map_err(http)?
                .json()
                .await
                .map_err(http)?;
            let done = page.len() < PAGE;
            all.extend(page);
            if done {
                break;
            }
        }
        all.sort_by(|a, b| a.start_time_local.cmp(&b.start_time_local));
        Ok(all)
    }

    /// The activity as TCX.
    pub async fn download_tcx(&self, activity_id: u64) -> AppResult<String> {
        self.client
            .get(format!(
                "{API_BASE}/download-service/export/tcx/activity/{activity_id}"
            ))
            .header(header::AUTHORIZATION, &self.bearer)
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(http)?
            .text()
            .await
            .map_err(http)
    }
}

/// Result of uploading one file.
#[derive(Debug, Clone, PartialEq)]
pub enum Upload {
    /// Created; Garmin's activity id when it reported one.
    Created(Option<u64>),
    /// Garmin already has this activity.
    Duplicate(Option<u64>),
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    #[serde(default)]
    detailed_import_result: UploadDetails,
}

#[derive(Debug, Default, Deserialize)]
struct UploadDetails {
    #[serde(default)]
    successes: Vec<UploadItem>,
    #[serde(default)]
    failures: Vec<UploadItem>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadItem {
    #[serde(default)]
    internal_id: Option<u64>,
    #[serde(default)]
    messages: Vec<UploadMessage>,
}

#[derive(Debug, Default, Deserialize)]
struct UploadMessage {
    #[serde(default)]
    content: String,
}

/// Interprets Garmin's upload response (HTTP 201/202 on success, 409 for duplicates).
fn upload_result(status: u16, body: &str) -> AppResult<Upload> {
    let parsed: UploadResponse = serde_json::from_str(body).unwrap_or_default();
    let details = parsed.detailed_import_result;
    if let Some(item) = details.successes.first() {
        return Ok(Upload::Created(item.internal_id));
    }
    let duplicate = details.failures.iter().find(|f| {
        f.messages
            .iter()
            .any(|m| m.content.to_ascii_lowercase().contains("duplicate"))
    });
    if status == 409 || duplicate.is_some() {
        return Ok(Upload::Duplicate(duplicate.and_then(|d| d.internal_id)));
    }
    if (200..300).contains(&status) {
        return Ok(Upload::Created(None));
    }
    let reason = details
        .failures
        .iter()
        .flat_map(|f| f.messages.iter().map(|m| m.content.clone()))
        .next()
        .unwrap_or_else(|| format!("HTTP {status}"));
    Err(garmin_error(format!("upload rejected: {reason}")))
}

impl Api {
    /// Uploads a TCX file to the signed-in account.
    pub async fn upload_tcx(&self, file_name: &str, xml: String) -> AppResult<Upload> {
        let part = reqwest::multipart::Part::text(xml)
            .file_name(file_name.to_string())
            .mime_str("application/octet-stream")
            .map_err(http)?;
        let response = self
            .client
            .post(format!("{API_BASE}/upload-service/upload"))
            .header(header::AUTHORIZATION, &self.bearer)
            .multipart(reqwest::multipart::Form::new().part("file", part))
            .send()
            .await
            .map_err(http)?;
        let status = response.status().as_u16();
        let body = response.text().await.map_err(http)?;
        upload_result(status, &body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_upload_responses() {
        let created =
            r#"{"detailedImportResult":{"successes":[{"internalId":18412345678}],"failures":[]}}"#;
        assert_eq!(
            upload_result(201, created).unwrap(),
            Upload::Created(Some(18_412_345_678))
        );
        let duplicate = r#"{"detailedImportResult":{"successes":[],"failures":[{"internalId":555,
            "messages":[{"code":202,"content":"Duplicate Activity."}]}]}}"#;
        assert_eq!(
            upload_result(409, duplicate).unwrap(),
            Upload::Duplicate(Some(555))
        );
        assert_eq!(upload_result(409, "").unwrap(), Upload::Duplicate(None));
        let rejected =
            r#"{"detailedImportResult":{"failures":[{"messages":[{"content":"Invalid file"}]}]}}"#;
        assert!(
            matches!(upload_result(400, rejected), Err(AppError::Invalid(m)) if m.contains("Invalid file"))
        );
    }

    #[test]
    fn reads_sign_in_pages() {
        let form = r#"<title>GARMIN Authentication Application</title>
            <input type="hidden" name="_csrf" value="82DF4F0A" />"#;
        assert_eq!(capture(&CSRF, form, "csrf").unwrap(), "82DF4F0A");
        let success = r#"<title>Success</title><script>var response_url =
            "https:\/\/sso.garmin.com\/sso\/embed?ticket=ST-0123-abc-cas";</script>"#;
        assert_eq!(capture(&TITLE, success, "title").unwrap(), "Success");
        assert_eq!(
            capture(&TICKET, success, "ticket").unwrap(),
            "ST-0123-abc-cas"
        );
        assert!(capture(&TICKET, form, "ticket").is_err());
    }

    #[test]
    fn activity_date_is_the_local_day() {
        let a = ActivitySummary {
            activity_id: 1,
            start_time_local: "2025-05-06 13:18:33".into(),
            activity_name: None,
        };
        assert_eq!(a.date(), "2025-05-06");
    }
}

/// Opt-in check that Garmin's sign-in pages still look as expected, using an address that
/// can't exist: `RUNNINGMATE_LIVE_GARMIN=1 cargo test garmin_live -- --ignored`.
#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "contacts sso.garmin.com"]
    async fn garmin_live_rejects_unknown_account() {
        if std::env::var_os("RUNNINGMATE_LIVE_GARMIN").is_none() {
            return;
        }
        match sign_in("nobody@example.invalid", "not-a-password").await {
            Err(AppError::Invalid(message)) => {
                assert!(message.contains("sign-in failed"), "{message}")
            }
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("an unknown account must not sign in"),
        }
    }
}
