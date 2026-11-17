//! Facts about one activity compared with activities over a similar distance and with the
//! recent weeks, for the AI summary. Every number and comparison is worked out here, already
//! phrased ("12 s/km faster than average"), so the language model only has to put it into words.

use std::path::Path;

use chrono::{DateTime, Local};
use rusqlite::Connection;
use serde::Serialize;

use super::{DAY_SECONDS, fitness, heart_rate_zones, segments, training_effect};
use crate::{
    activity_type::ViewMode,
    db::{
        activities::{self, ActivityFilter, ActivityQuery, ActivityRow, SortDirection, SortField},
        performances, weather,
    },
    error::AppResult,
};

/// Activities within this share of the distance count as similar…
const SIMILAR_SHARE: f64 = 0.1;
/// …but at least within this many km, so short runs still find company.
const SIMILAR_MIN_KM: f64 = 0.5;
/// Most recent similar activities compared with.
const SIMILAR_LIMIT: usize = 10;
/// Upper bound on the activities read for one group.
const MAX_ROWS: i64 = 1000;
/// Splits need at least this many segments to say something about pacing.
const MIN_SPLIT_SEGMENTS: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparisonFacts {
    pub activity: ActivityFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar_distance: Option<GroupFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_weeks: Option<GroupFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splits: Option<SplitFacts>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_effect: Option<TrainingEffectFacts>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub heart_rate_zones: Vec<ZoneShare>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form: Option<FormFacts>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub best_performances: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActivityFacts {
    pub sport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The user's own note about the activity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub date: String,
    pub distance_km: f64,
    pub duration: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_kmh: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_heart_rate_bpm: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation_gain_m: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_temperature_c: Option<i64>,
}

/// The activity against a group of earlier activities of the same sport.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GroupFacts {
    /// Which activities the group holds.
    pub selection: String,
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities_per_week: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub km_per_week: Option<f64>,
    pub average_distance_km: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_rank: Option<String>,
    /// Pace, or speed for rides.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_pace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_speed_kmh: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub this_activity_vs_average: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_rank: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_heart_rate_bpm: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heart_rate_vs_average: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_elevation_gain_m: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elevation_vs_average: Option<String>,
    /// Speed per heartbeat; more is better aerobic efficiency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efficiency_vs_average: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SplitFacts {
    pub first_half_pace: String,
    pub second_half_pace: String,
    pub pacing: String,
    pub fastest_segment: String,
    pub slowest_segment: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrainingEffectFacts {
    /// 0.0–5.0 from the peak EPOC, as on Garmin watches.
    pub aerobic_training_effect: f64,
    pub aerobic_meaning: &'static str,
    /// 0.0–5.0 from the time near the maximum heart rate.
    pub anaerobic_training_effect: f64,
    pub anaerobic_meaning: &'static str,
    pub peak_epoc_ml_per_kg: f64,
    pub recovery_hours: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ZoneShare {
    pub zone: u8,
    pub percent: i64,
}

/// Training form on the day, in Joe Friel's zones as the health page shows them.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FormFacts {
    pub form_percent_of_fitness: i64,
    pub zone: &'static str,
    pub meaning: &'static str,
}

/// One activity's comparable values.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    distance: f64,
    /// km/h.
    speed: Option<f64>,
    heart_rate: Option<f64>,
    elevation: Option<f64>,
}

impl Sample {
    fn from_row(row: &ActivityRow) -> Option<Self> {
        let distance = row.distance.filter(|d| *d > 0.0)?;
        Some(Self {
            distance,
            speed: speed(row.avg_speed, distance, row.duration),
            heart_rate: row.avg_heart_rate.filter(|h| *h > 0).map(|h| h as f64),
            elevation: row.elevation_gain,
        })
    }

    /// Speed per beat; only with both values.
    fn efficiency(&self) -> Option<f64> {
        Some(self.speed? / self.heart_rate?)
    }
}

/// The stored average speed, or distance over duration when there is none.
fn speed(avg_speed: Option<f64>, distance: f64, duration: Option<i64>) -> Option<f64> {
    avg_speed.filter(|s| *s > 0.0).or_else(|| {
        duration
            .filter(|d| *d > 0)
            .map(|d| distance / (d as f64 / 3600.0))
    })
}

/// "m:ss /km" for a speed in km/h.
fn pace_text(speed_kmh: f64) -> String {
    pace_text_min(60.0 / speed_kmh)
}

/// "m:ss /km" for a pace in min/km.
fn pace_text_min(min_per_km: f64) -> String {
    let seconds = (min_per_km * 60.0).round() as i64;
    format!("{}:{:02} /km", seconds / 60, seconds % 60)
}

fn duration_text(seconds: i64) -> String {
    let (h, m, s) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let (sum, count) = values.fold((0.0, 0), |(s, c), v| (s + v, c + 1));
    (count > 0).then(|| sum / count as f64)
}

/// "2 of 11 (1 = fastest)": the activity's place among the group plus itself.
fn rank(own: f64, others: impl Iterator<Item = f64>, best: &str) -> String {
    let mut count = 1;
    let mut place = 1;
    for value in others {
        count += 1;
        if value > own {
            place += 1;
        }
    }
    format!("{place} of {count} (1 = {best})")
}

/// Pace (or for rides speed) against the group mean, in words.
fn speed_vs_average(view: ViewMode, own: f64, average: f64) -> String {
    if view == ViewMode::Cycling {
        let delta = round1(own - average);
        return match delta {
            d if d >= 0.5 => format!("{d} km/h faster than average"),
            d if d <= -0.5 => format!("{} km/h slower than average", -d),
            _ => "about the same speed as average".into(),
        };
    }
    let seconds = ((60.0 / own - 60.0 / average) * 60.0).round() as i64;
    match seconds {
        s if s <= -3 => format!("{} s/km faster than average", -s),
        s if s >= 3 => format!("{s} s/km slower than average"),
        _ => "about the same pace as average".into(),
    }
}

fn heart_rate_vs_average(own: f64, average: f64) -> String {
    let delta = (own - average).round() as i64;
    match delta {
        d if d >= 2 => format!("{d} bpm higher than average"),
        d if d <= -2 => format!("{} bpm lower than average", -d),
        _ => "about the same as average".into(),
    }
}

fn elevation_vs_average(own: f64, average: f64) -> String {
    let delta = (own - average).round() as i64;
    // Within 10 % (at least 10 m) counts as the same.
    if delta.abs() < (average * 0.1).max(10.0) as i64 {
        return "about the same as average".into();
    }
    if delta > 0 {
        format!("{delta} m more climbing than average")
    } else {
        format!("{} m less climbing than average", -delta)
    }
}

fn efficiency_vs_average(own: f64, average: f64) -> String {
    let percent = ((own / average - 1.0) * 100.0).round() as i64;
    match percent {
        p if p >= 2 => format!("{p}% better than average"),
        p if p <= -2 => format!("{}% worse than average", -p),
        _ => "about the same as average".into(),
    }
}

/// Compares `own` with `group`; `None` for an empty group.
fn group_facts(
    view: ViewMode,
    selection: String,
    own: &Sample,
    group: &[Sample],
    weeks: Option<u8>,
) -> Option<GroupFacts> {
    if group.is_empty() {
        return None;
    }
    let speeds: Vec<f64> = group.iter().filter_map(|s| s.speed).collect();
    let average_speed = mean(speeds.iter().copied());
    let average_heart_rate = mean(group.iter().filter_map(|s| s.heart_rate));
    let average_efficiency = mean(group.iter().filter_map(Sample::efficiency));
    let average_elevation = mean(group.iter().filter_map(|s| s.elevation));
    let cycling = view == ViewMode::Cycling;
    let total_km: f64 = group.iter().map(|s| s.distance).sum::<f64>() + own.distance;
    Some(GroupFacts {
        selection,
        count: group.len(),
        // The period's totals include the activity itself.
        activities_per_week: weeks.map(|w| round1((group.len() + 1) as f64 / w as f64)),
        km_per_week: weeks.map(|w| round1(total_km / w as f64)),
        average_distance_km: round1(mean(group.iter().map(|s| s.distance)).unwrap_or_default()),
        distance_rank: weeks
            .map(|_| rank(own.distance, group.iter().map(|s| s.distance), "longest")),
        average_pace: average_speed.filter(|_| !cycling).map(pace_text),
        average_speed_kmh: average_speed.filter(|_| cycling).map(round1),
        this_activity_vs_average: own
            .speed
            .zip(average_speed)
            .map(|(o, a)| speed_vs_average(view, o, a)),
        speed_rank: own
            .speed
            .filter(|_| !speeds.is_empty())
            .map(|o| rank(o, speeds.iter().copied(), "fastest")),
        average_heart_rate_bpm: average_heart_rate.map(|h| h.round() as i64),
        heart_rate_vs_average: own
            .heart_rate
            .zip(average_heart_rate)
            .map(|(o, a)| heart_rate_vs_average(o, a)),
        average_elevation_gain_m: average_elevation.map(|e| e.round() as i64),
        elevation_vs_average: own
            .elevation
            .zip(average_elevation)
            .map(|(o, a)| elevation_vs_average(o, a)),
        efficiency_vs_average: own
            .efficiency()
            .zip(average_efficiency)
            .map(|(o, a)| efficiency_vs_average(o, a)),
    })
}

/// Pacing from per-segment paces (min/km) and distances (km), for runs and walks.
fn split_facts(segments: &[(f64, f64)]) -> Option<SplitFacts> {
    if segments.len() < MIN_SPLIT_SEGMENTS {
        return None;
    }
    let total: f64 = segments.iter().map(|(_, d)| d).sum();
    let (mut first, mut second) = ((0.0, 0.0), (0.0, 0.0));
    let mut covered = 0.0;
    for (pace, distance) in segments {
        let half = if covered + distance / 2.0 <= total / 2.0 {
            &mut first
        } else {
            &mut second
        };
        half.0 += pace * distance;
        half.1 += distance;
        covered += distance;
    }
    if first.1 <= 0.0 || second.1 <= 0.0 {
        return None;
    }
    let (first_pace, second_pace) = (first.0 / first.1, second.0 / second.1);
    let seconds = ((second_pace - first_pace) * 60.0).round() as i64;
    let pacing = match seconds {
        s if s <= -3 => format!("negative split: second half {} s/km faster", -s),
        s if s >= 3 => format!("positive split: second half {s} s/km slower"),
        _ => "even pacing".into(),
    };
    // The last segment is often a short remainder, so only full-length ones count here.
    let full = segments
        .iter()
        .enumerate()
        .filter(|(_, (_, d))| *d >= 0.9 * segments[0].1);
    let by_pace = |a: &(usize, &(f64, f64)), b: &(usize, &(f64, f64))| a.1.0.total_cmp(&b.1.0);
    let segment = |(i, (pace, _)): (usize, &(f64, f64))| {
        format!("segment {} at {}", i + 1, pace_text_min(*pace))
    };
    Some(SplitFacts {
        first_half_pace: pace_text_min(first_pace),
        second_half_pace: pace_text_min(second_pace),
        pacing,
        fastest_segment: full.clone().min_by(by_pace).map(segment)?,
        slowest_segment: full.max_by(by_pace).map(segment)?,
    })
}

/// Form zone and its meaning for form in percent of fitness, as `FORM_SCALE` on the health page.
fn form_zone(percent: f64) -> (&'static str, &'static str) {
    match percent {
        p if p < -30.0 => (
            "High risk",
            "Fatigue is far above fitness; too much of this risks overtraining.",
        ),
        p if p < -10.0 => (
            "Optimal",
            "A productive training load: fitness is building.",
        ),
        p if p < 5.0 => (
            "Grey zone",
            "Maintaining: not much building, not much recovering.",
        ),
        p if p < 20.0 => ("Fresh", "Rested and fit: a good time for a race or a test."),
        _ => (
            "Transition",
            "Very rested: without training fitness starts to drop.",
        ),
    }
}

fn ordinal(place: usize) -> String {
    match place {
        1 => "fastest".into(),
        2 => "2nd fastest".into(),
        3 => "3rd fastest".into(),
        n => format!("{n}th fastest"),
    }
}

fn earlier(
    conn: &Connection,
    view: ViewMode,
    activity_type: &str,
    filter: ActivityFilter,
    limit: usize,
) -> AppResult<Vec<Sample>> {
    let page = activities::list(
        conn,
        &ActivityQuery {
            view_mode: view,
            offset: 0,
            // `All` has no type filter of its own, so read more and keep the same type.
            limit: if view == ViewMode::All {
                MAX_ROWS
            } else {
                limit.min(MAX_ROWS as usize) as i64
            },
            sort_field: SortField::Date,
            sort_direction: SortDirection::Desc,
            filter,
        },
    )?;
    Ok(page
        .rows
        .iter()
        .filter(|r| view != ViewMode::All || r.activity_type.as_deref() == Some(activity_type))
        .filter_map(Sample::from_row)
        .take(limit)
        .collect())
}

/// Everything the summary is written from. Only activities before this one are compared, so
/// the facts stay the same when an older activity is summarised again later.
pub fn comparison(
    conn: &Connection,
    id: i64,
    imports_dir: &Path,
    period_weeks: u8,
) -> AppResult<ComparisonFacts> {
    let detail = activities::get(conn, id)?;
    let view = detail.view_mode;
    let date = detail.date.unwrap_or_default();
    let activity_type = detail.activity_type.clone().unwrap_or_default();
    let distance = detail.distance.unwrap_or_default();
    let own = Sample {
        distance,
        speed: speed(detail.avg_speed, distance, detail.duration).filter(|_| distance > 0.0),
        heart_rate: detail.avg_heart_rate.filter(|h| *h > 0).map(|h| h as f64),
        elevation: detail.elevation_gain,
    };
    let cycling = view == ViewMode::Cycling;
    let sport = match view {
        ViewMode::All => activity_type.clone(),
        _ => serde_json::to_value(view)?
            .as_str()
            .unwrap_or_default()
            .to_string(),
    };

    let margin = (distance * SIMILAR_SHARE).max(SIMILAR_MIN_KM);
    let similar = earlier(
        conn,
        view,
        &activity_type,
        ActivityFilter {
            min_distance: Some(distance - margin),
            max_distance: Some(distance + margin),
            max_date: Some(date - 1),
            ..Default::default()
        },
        SIMILAR_LIMIT,
    )?;
    let recent = earlier(
        conn,
        view,
        &activity_type,
        ActivityFilter {
            min_date: Some(date - i64::from(period_weeks) * 7 * DAY_SECONDS),
            max_date: Some(date - 1),
            ..Default::default()
        },
        usize::MAX,
    )?;
    let sport_lower = sport.to_lowercase();

    let segment_rows = segments(conn, id)?;
    let splits = if matches!(view, ViewMode::Running | ViewMode::Walking) {
        split_facts(
            &segment_rows
                .iter()
                .filter_map(|s| {
                    Some((
                        s.pace.filter(|p| *p > 0.0)?,
                        s.distance.filter(|d| *d > 0.0)?,
                    ))
                })
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let effect = training_effect(conn, id, imports_dir)?.map(|e| TrainingEffectFacts {
        aerobic_training_effect: e.aerobic,
        aerobic_meaning: super::training_effect::label(e.aerobic),
        anaerobic_training_effect: e.anaerobic,
        anaerobic_meaning: super::training_effect::label(e.anaerobic),
        peak_epoc_ml_per_kg: e.peak_epoc,
        recovery_hours: e.recovery_hours.round(),
    });
    let zones = heart_rate_zones(conn, id, imports_dir)?
        .unwrap_or_default()
        .into_iter()
        .map(|z| ZoneShare {
            zone: z.zone,
            percent: z.percent.round() as i64,
        })
        .filter(|z| z.percent > 0)
        .collect();

    let utc_day = DateTime::from_timestamp(date, 0)
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let form = super::fitness_until(conn, date)?
        .pop()
        .filter(|day| day.day == utc_day && day.fitness > 0.0)
        .map(|day: fitness::FitnessDay| {
            let percent = day.form / day.fitness * 100.0;
            let (zone, meaning) = form_zone(percent);
            FormFacts {
                form_percent_of_fitness: percent.round() as i64,
                zone,
                meaning,
            }
        });

    let mut best_performances = Vec::new();
    if view != ViewMode::All {
        let top = performances::top(conn, view)?;
        for best in performances::for_activity(conn, id)? {
            let place = top
                .iter()
                .filter(|t| t.distance == best.distance)
                .position(|t| t.activity_id == id);
            if let Some(place) = place {
                best_performances.push(format!(
                    "{} {} km in your records ({})",
                    ordinal(place + 1),
                    best.distance,
                    pace_text_min(best.pace)
                ));
            }
        }
    }

    Ok(ComparisonFacts {
        activity: ActivityFacts {
            sport,
            kind: detail
                .type_detail
                .filter(|k| !k.is_empty() && *k != activity_type),
            title: detail.title.filter(|t| !t.trim().is_empty()),
            comment: detail.comment.filter(|c| !c.trim().is_empty()),
            date: DateTime::from_timestamp(date, 0)
                .map(|t| t.with_timezone(&Local).format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            distance_km: (distance * 100.0).round() / 100.0,
            duration: duration_text(detail.duration.unwrap_or_default()),
            pace: own.speed.filter(|_| !cycling).map(pace_text),
            speed_kmh: own.speed.filter(|_| cycling).map(round1),
            avg_heart_rate_bpm: detail.avg_heart_rate.filter(|h| *h > 0),
            elevation_gain_m: own.elevation.map(|e| e.round() as i64),
            max_temperature_c: weather::get(conn, id)?
                .and_then(|w| w.max_temp)
                .map(|t| t.round() as i64),
        },
        similar_distance: group_facts(
            view,
            format!(
                "the last {} {sport_lower} activities of {:.1}–{:.1} km before this one",
                similar.len(),
                (distance - margin).max(0.0),
                distance + margin
            ),
            &own,
            &similar,
            None,
        ),
        recent_weeks: group_facts(
            view,
            format!("all {sport_lower} activities in the {period_weeks} weeks before this one"),
            &own,
            &recent,
            Some(period_weeks),
        ),
        splits,
        training_effect: effect,
        heart_rate_zones: zones,
        form,
        best_performances,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Db, activities::tests::insert_activity};

    const DAY: i64 = DAY_SECONDS;
    const NOW: i64 = 1_750_000_000;

    fn set_speed(conn: &Connection, id: i64, speed: f64, heart_rate: i64) {
        conn.execute(
            "UPDATE runs SET avg_speed = ?1, avg_heart_rate = ?2 WHERE activity_id = ?3",
            rusqlite::params![speed, heart_rate, id],
        )
        .unwrap();
    }

    fn facts(conn: &Connection, id: i64, weeks: u8) -> ComparisonFacts {
        comparison(conn, id, Path::new("/nonexistent"), weeks).unwrap()
    }

    #[test]
    fn similar_distance_takes_earlier_runs_within_ten_percent() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Running", NOW, 10.0, "Today");
        insert_activity(&conn, 2, "Running", NOW - 3 * DAY, 9.2, "In range");
        insert_activity(
            &conn,
            3,
            "Trailrun",
            NOW - 5 * DAY,
            10.8,
            "In range, other run type",
        );
        insert_activity(&conn, 4, "Running", NOW - 6 * DAY, 12.0, "Too long");
        insert_activity(&conn, 5, "Running", NOW + DAY, 10.0, "Later");
        insert_activity(&conn, 6, "Biking", NOW - DAY, 10.0, "Other sport");
        set_speed(&conn, 1, 12.0, 150);
        set_speed(&conn, 2, 10.0, 150);
        set_speed(&conn, 3, 10.0, 150);

        let similar = facts(&conn, 1, 4).similar_distance.unwrap();
        assert_eq!(similar.count, 2);
        assert_eq!(similar.average_distance_km, 10.0);
        assert_eq!(similar.average_pace.as_deref(), Some("6:00 /km"));
        // 5:00 against 6:00 /km.
        assert_eq!(
            similar.this_activity_vs_average.as_deref(),
            Some("60 s/km faster than average")
        );
        assert_eq!(similar.speed_rank.as_deref(), Some("1 of 3 (1 = fastest)"));
        assert_eq!(
            similar.heart_rate_vs_average.as_deref(),
            Some("about the same as average")
        );
        assert_eq!(
            similar.efficiency_vs_average.as_deref(),
            Some("20% better than average")
        );
        assert_eq!(similar.distance_rank, None);
    }

    #[test]
    fn recent_weeks_cover_the_period_before_the_activity() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Running", NOW, 5.0, "Today");
        insert_activity(&conn, 2, "Running", NOW - 10 * DAY, 15.0, "Inside");
        insert_activity(&conn, 3, "Running", NOW - 20 * DAY, 8.0, "Inside");
        insert_activity(&conn, 4, "Running", NOW - 40 * DAY, 30.0, "Outside");

        let facts = facts(&conn, 1, 4);
        let recent = facts.recent_weeks.unwrap();
        assert_eq!(recent.count, 2);
        assert_eq!(recent.activities_per_week, Some(0.8));
        assert_eq!(recent.km_per_week, Some(7.0));
        assert_eq!(
            recent.distance_rank.as_deref(),
            Some("3 of 3 (1 = longest)")
        );
        assert_eq!(facts.similar_distance, None);
    }

    #[test]
    fn leaves_out_what_is_missing() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Running", NOW, 10.0, "Alone");
        conn.execute("UPDATE runs SET avg_heart_rate = NULL", [])
            .unwrap();

        let facts = facts(&conn, 1, 4);
        assert_eq!(facts.activity.pace.as_deref(), Some("6:00 /km"));
        assert_eq!(facts.activity.duration, "1:00:00");
        assert_eq!(facts.activity.avg_heart_rate_bpm, None);
        assert_eq!(facts.similar_distance, None);
        assert_eq!(facts.recent_weeks, None);
        assert_eq!(facts.splits, None);
        assert_eq!(facts.training_effect, None);
        assert!(facts.heart_rate_zones.is_empty());
        let json = serde_json::to_value(&facts).unwrap();
        assert!(json.get("similar_distance").is_none());
        assert!(json["activity"].get("avg_heart_rate_bpm").is_none());
    }

    #[test]
    fn rides_compare_speed() {
        let db = Db::open_in_memory().unwrap();
        let conn = db.conn().unwrap();
        insert_activity(&conn, 1, "Biking", NOW, 40.0, "Today");
        insert_activity(&conn, 2, "Biking", NOW - DAY, 40.0, "Yesterday");
        conn.execute(
            "UPDATE cycling SET avg_speed = 25 WHERE activity_id = 1",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE cycling SET avg_speed = 22 WHERE activity_id = 2",
            [],
        )
        .unwrap();

        let facts = facts(&conn, 1, 2);
        assert_eq!(facts.activity.pace, None);
        assert_eq!(facts.activity.speed_kmh, Some(25.0));
        let similar = facts.similar_distance.unwrap();
        assert_eq!(similar.average_speed_kmh, Some(22.0));
        assert_eq!(
            similar.this_activity_vs_average.as_deref(),
            Some("3 km/h faster than average")
        );
    }

    #[test]
    fn splits_compare_the_halves() {
        let paces = [(6.0, 1.0), (5.9, 1.0), (5.6, 1.0), (5.5, 1.0), (5.8, 0.3)];
        let splits = split_facts(&paces).unwrap();
        assert_eq!(splits.first_half_pace, "5:57 /km");
        assert_eq!(splits.pacing, "negative split: second half 22 s/km faster");
        assert_eq!(splits.fastest_segment, "segment 4 at 5:30 /km");
        assert_eq!(splits.slowest_segment, "segment 1 at 6:00 /km");
        assert_eq!(split_facts(&paces[..3]), None);
    }

    #[test]
    fn elevation_counts_small_differences_as_the_same() {
        assert_eq!(
            elevation_vs_average(55.0, 50.0),
            "about the same as average"
        );
        assert_eq!(
            elevation_vs_average(240.0, 60.0),
            "180 m more climbing than average"
        );
        assert_eq!(
            elevation_vs_average(20.0, 200.0),
            "180 m less climbing than average"
        );
    }

    #[test]
    fn form_zones_match_the_health_page() {
        assert_eq!(form_zone(-35.0).0, "High risk");
        assert_eq!(form_zone(-20.0).0, "Optimal");
        assert_eq!(form_zone(0.0).0, "Grey zone");
        assert_eq!(form_zone(10.0).0, "Fresh");
        assert_eq!(form_zone(25.0).0, "Transition");
    }

    #[test]
    fn formats_pace_and_duration() {
        assert_eq!(pace_text(12.0), "5:00 /km");
        assert_eq!(pace_text_min(5.75), "5:45 /km");
        assert_eq!(duration_text(2710), "45:10");
        assert_eq!(duration_text(3723), "1:02:03");
    }
}
