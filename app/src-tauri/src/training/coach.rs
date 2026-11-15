//! Facts about a plan week or a session for the optional local AI model, and its instructions.
//! The plan itself never depends on the model; it only explains and comments.

use serde::Serialize;
use serde_json::{Value, json};

use super::{generator::TrainingTarget, paces::PaceRange};
use crate::{
    ai,
    analysis::comparison::ComparisonFacts,
    db::training::{SessionStatus, TrainingPlan},
    error::AppResult,
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PlanFacts {
    pub target: String,
    pub weeks: u32,
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionFacts {
    pub date: String,
    pub kind: String,
    pub planned_km: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_pace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub note: String,
    pub verdict: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_km: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_pace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fastest_km_pace: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub heart_rate_above_zone_2: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeekFacts {
    pub plan: PlanFacts,
    /// 1-based.
    pub week: u32,
    pub phase: String,
    pub sessions: Vec<SessionFacts>,
    pub previous_week: Vec<SessionFacts>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionCoachFacts {
    pub plan: PlanFacts,
    pub session: SessionFacts,
    /// The run compared with earlier ones, as for the activity summary.
    pub run: ComparisonFacts,
}

/// "5:07 /km".
pub fn pace_text(seconds: f64) -> String {
    let total = seconds.round() as i64;
    format!("{}:{:02} /km", total / 60, total % 60)
}

fn range_text(range: PaceRange) -> String {
    let fast = pace_text(range.fast);
    format!(
        "{}–{}",
        fast.trim_end_matches(" /km"),
        pace_text(range.slow)
    )
}

fn name<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_default()
}

pub fn plan_facts(plan: &TrainingPlan) -> PlanFacts {
    let target = match plan.target {
        TrainingTarget::Race => {
            let distance = format!("race over {:.1} km", plan.distance.unwrap_or(0.0));
            match plan.target_time {
                Some(time) => format!(
                    "{distance} in {}:{:02}:{:02}",
                    time / 3600,
                    time / 60 % 60,
                    time % 60
                ),
                None => distance,
            }
        }
        TrainingTarget::Shape => "getting in shape".into(),
        TrainingTarget::Fitness => "more fitness".into(),
    };
    PlanFacts {
        target,
        weeks: plan.weeks,
        start: plan.start.clone(),
        end: plan.end.clone(),
    }
}

pub fn session_facts(status: &SessionStatus) -> SessionFacts {
    let SessionStatus {
        session,
        run,
        check,
    } = status;
    let work = session.structure.map(|s| {
        let each = match (s.metres, s.seconds) {
            (Some(m), _) => format!("{m} m"),
            (None, Some(sec)) => format!("{} min", sec / 60),
            _ => String::new(),
        };
        format!("{} × {each}, {} s rest", s.reps, s.rest)
    });
    SessionFacts {
        date: session.date.clone(),
        kind: name(&session.kind),
        planned_km: session.distance,
        planned_pace: session.pace.map(range_text),
        work,
        note: session.note.clone(),
        verdict: name(&check.verdict),
        done_km: run.as_ref().map(|r| r.distance),
        done_pace: run.as_ref().and_then(|r| r.pace).map(pace_text),
        fastest_km_pace: check.work_pace.map(pace_text),
        heart_rate_above_zone_2: check.above_zone_two,
    }
}

/// Instructions for explaining a week of the plan.
fn week_prompt(language: &str) -> String {
    format!(
        "You are a friendly, knowledgeable running coach. You get a training plan's target and \
         one week of it as JSON, with last week's sessions and how they went (verdict: done, \
         partial, tooHard, tooEasy, missed, skipped or planned).\n\
         Write 3 to 5 sentences in {language}, as plain text without headings, lists or \
         markdown: what this week is for in the plan's phase, what matters in its key sessions, \
         and, if last week shows a pattern (missed sessions, easy runs too hard), one hint about \
         it.\n\
         Rules: use only the facts in the JSON and never calculate new numbers. For pace a lower \
         value is faster. Don't give medical advice. Address the athlete as \"you\".",
        language = ai::language_name(language)
    )
}

/// Instructions for commenting on how a run did a session.
fn session_prompt(language: &str) -> String {
    format!(
        "You are a friendly, knowledgeable running coach. You get a planned training session \
         and the run that did it as JSON: what was planned, the verdict (done, partial, tooHard, \
         tooEasy) and the run compared with earlier runs.\n\
         Write 3 to 4 sentences in {language}, as plain text without headings, lists or \
         markdown: did the run serve the purpose of the session, and what to keep or change \
         next time.\n\
         Rules: use only the facts in the JSON and never calculate new numbers. For pace a lower \
         value is faster; an easy run is meant to be easy, a tempo or interval run meant to hit \
         its pace. Don't give medical advice. Address the athlete as \"you\".",
        language = ai::language_name(language)
    )
}

fn request(model: &str, system: String, facts: &impl Serialize) -> AppResult<Value> {
    Ok(json!({
        "model": model,
        "stream": false,
        "options": { "temperature": 0.3 },
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": serde_json::to_string_pretty(facts)? },
        ],
    }))
}

pub fn week_request(model: &str, facts: &WeekFacts, language: &str) -> AppResult<Value> {
    request(model, week_prompt(language), facts)
}

pub fn session_request(model: &str, facts: &SessionCoachFacts, language: &str) -> AppResult<Value> {
    request(model, session_prompt(language), facts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_paces_as_minutes() {
        assert_eq!(pace_text(307.4), "5:07 /km");
        assert_eq!(
            range_text(PaceRange {
                fast: 230.0,
                slow: 236.0
            }),
            "3:50–3:56 /km"
        );
    }

    #[test]
    fn builds_the_week_request() {
        let facts = WeekFacts {
            plan: PlanFacts {
                target: "race over 10.0 km".into(),
                weeks: 8,
                start: "2026-11-16".into(),
                end: "2027-01-10".into(),
            },
            week: 3,
            phase: "build".into(),
            sessions: Vec::new(),
            previous_week: Vec::new(),
        };
        let body = week_request("qwen2.5:7b", &facts, "de").unwrap();
        assert!(
            body["messages"][0]["content"]
                .as_str()
                .unwrap()
                .contains("in German")
        );
        let user: Value =
            serde_json::from_str(body["messages"][1]["content"].as_str().unwrap()).unwrap();
        assert_eq!(user["phase"], "build");
    }
}
