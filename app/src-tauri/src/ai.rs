//! Activity summaries from a local Ollama server (https://ollama.com). The model only gets the
//! precomputed [`ComparisonFacts`] and is told not to calculate anything itself.

use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Semaphore;

use crate::{
    analysis::comparison::ComparisonFacts,
    error::{AppError, AppResult},
};

/// Small local models take a while for the first answer, when they are loaded into memory.
const TIMEOUT: Duration = Duration::from_secs(180);

/// Lets one summary run at a time, so an import doesn't send Ollama a burst of requests.
pub struct AiQueue(pub Semaphore);

impl Default for AiQueue {
    fn default() -> Self {
        Self(Semaphore::new(1))
    }
}

fn http(e: reqwest::Error) -> AppError {
    AppError::Invalid(format!("Ollama: {e}"))
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .build()
        .map_err(http)
}

fn endpoint(url: &str, path: &str) -> String {
    format!("{}/api/{path}", url.trim_end_matches('/'))
}

#[derive(Deserialize)]
struct Tags {
    models: Vec<Tag>,
}

#[derive(Deserialize)]
struct Tag {
    name: String,
}

/// Names of the models installed on the server; also serves as the connection check.
pub async fn list_models(url: &str) -> AppResult<Vec<String>> {
    let tags: Tags = client()?
        .get(endpoint(url, "tags"))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .json()
        .await
        .map_err(http)?;
    let mut names: Vec<String> = tags.models.into_iter().map(|t| t.name).collect();
    names.sort();
    Ok(names)
}

pub fn language_name(code: &str) -> &'static str {
    match code {
        "de" => "German",
        "fr" => "French",
        _ => "English",
    }
}

/// Instructions for the model.
pub fn system_prompt(language: &str) -> String {
    format!(
        "You are a friendly, knowledgeable endurance coach. You get facts about one activity \
         as JSON: the activity itself, how it compares with earlier activities over a similar \
         distance and with the recent weeks, and optionally pacing, training effect, heart-rate \
         zones, training form and personal records.\n\
         Write a short summary of 4 to 6 sentences in {language}, as plain text without \
         headings, lists or markdown. Say what stands out compared with the earlier activities, \
         then give one practical hint for the next training.\n\
         Rules: use only the facts in the JSON and never calculate new numbers. For pace a lower \
         value is faster; more climbing or heat can explain a slower pace. Call it \"this activity\" or use its \
         date, never \"today\". Leave out what the JSON doesn't contain. Don't give medical advice. \
         Address the athlete as \"you\".",
        language = language_name(language)
    )
}

/// Request body for `POST /api/chat`.
pub fn chat_request(model: &str, facts: &ComparisonFacts, language: &str) -> AppResult<Value> {
    Ok(json!({
        "model": model,
        "stream": false,
        "options": { "temperature": 0.3 },
        "messages": [
            { "role": "system", "content": system_prompt(language) },
            { "role": "user", "content": serde_json::to_string_pretty(facts)? },
        ],
    }))
}

#[derive(Deserialize)]
struct ChatResponse {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: String,
}

/// Asks the model for a summary of the facts.
pub async fn summarize(
    url: &str,
    model: &str,
    facts: &ComparisonFacts,
    language: &str,
) -> AppResult<String> {
    chat(url, model, &chat_request(model, facts, language)?).await
}

/// Sends a `POST /api/chat` request body and returns the answer.
pub async fn chat(url: &str, model: &str, body: &Value) -> AppResult<String> {
    let response: ChatResponse = client()?
        .post(endpoint(url, "chat"))
        .json(body)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(http)?
        .json()
        .await
        .map_err(http)?;
    let text = response.message.content.trim().to_string();
    if text.is_empty() {
        return Err(AppError::Invalid(format!(
            "Ollama: {model} returned no text"
        )));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::comparison::ActivityFacts;

    fn facts() -> ComparisonFacts {
        ComparisonFacts {
            activity: ActivityFacts {
                sport: "Running".into(),
                kind: None,
                title: None,
                comment: None,
                date: "2026-11-14".into(),
                distance_km: 10.0,
                duration: "50:00".into(),
                pace: Some("5:00 /km".into()),
                speed_kmh: None,
                avg_heart_rate_bpm: Some(150),
                elevation_gain_m: None,
                max_temperature_c: None,
            },
            similar_distance: None,
            recent_weeks: None,
            splits: None,
            training_effect: None,
            heart_rate_zones: Vec::new(),
            form: None,
            best_performances: Vec::new(),
        }
    }

    #[test]
    fn builds_the_chat_request() {
        let body = chat_request("qwen2.5:7b", &facts(), "de").unwrap();
        assert_eq!(body["model"], "qwen2.5:7b");
        assert_eq!(body["stream"], false);
        let system = body["messages"][0]["content"].as_str().unwrap();
        assert!(system.contains("in German"));
        let user: Value =
            serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(user["activity"]["pace"], "5:00 /km");
        assert!(user["activity"].get("speed_kmh").is_none());
    }

    #[test]
    fn endpoint_ignores_a_trailing_slash() {
        assert_eq!(
            endpoint("http://localhost:11434/", "tags"),
            "http://localhost:11434/api/tags"
        );
    }

    /// Needs a running Ollama server with `qwen2.5:7b`:
    /// `cargo test ollama_live -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn ollama_live() {
        let url = "http://localhost:11434";
        println!("{:?}", list_models(url).await.unwrap());
        println!(
            "{}",
            summarize(url, "qwen2.5:7b", &facts(), "en").await.unwrap()
        );
    }

    /// Summarises the latest run of a copy of a real database:
    /// `RUNNINGMATE_TEST_DB=/path/to/copy.db cargo test ollama_real_run -- --ignored --nocapture`
    /// (the model can be changed with `RUNNINGMATE_TEST_MODEL`).
    #[tokio::test]
    #[ignore = "needs RUNNINGMATE_TEST_DB and a running Ollama server"]
    async fn ollama_real_run() {
        use crate::{
            activity_type::ViewMode,
            analysis::comparison,
            db::{
                Db,
                activities::{self, ActivityQuery, SortDirection, SortField},
            },
            paths,
        };
        let path = std::env::var("RUNNINGMATE_TEST_DB").expect("RUNNINGMATE_TEST_DB");
        let model = std::env::var("RUNNINGMATE_TEST_MODEL").unwrap_or("qwen2.5:14b".into());
        let facts = {
            let db = Db::open(std::path::Path::new(&path)).unwrap();
            let conn = db.conn().unwrap();
            let latest = activities::list(
                &conn,
                &ActivityQuery {
                    view_mode: ViewMode::Running,
                    offset: 0,
                    limit: 1,
                    sort_field: SortField::Date,
                    sort_direction: SortDirection::Desc,
                    filter: Default::default(),
                },
            )
            .unwrap()
            .rows[0]
                .id;
            comparison::comparison(&conn, latest, &paths::imports_dir().unwrap(), 4).unwrap()
        };
        println!("{}", serde_json::to_string_pretty(&facts).unwrap());
        for language in ["en", "de"] {
            let started = std::time::Instant::now();
            let text = summarize("http://localhost:11434", &model, &facts, language)
                .await
                .unwrap();
            println!("\n[{language}, {:.0?}]\n{text}", started.elapsed());
        }
    }
}
