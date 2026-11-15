# Training

A training plan gets you to a target over a number of weeks: a race (5 km, 10 km, half marathon, marathon or any other
distance, with an optional target time), getting in shape, or more fitness. The plan is built from fixed rules modelled
on common running plans, starting from your runs of the last 6 months. Plans are for running only.

## Making a plan
Open the **Training** page from the sidebar or with ⇧⌘T and click **New training plan**. The wizard has three steps.

1. **Target**: a race and its distance, *Get in shape* (mostly easy runs and one faster run a week) or *More fitness*
   (intervals and tempo runs). For a race you can give a target time as H:MM:SS; it sets the race pace. Without it the
   pace comes from your runs.
2. **Schedule**: the start date, the number of weeks (4, 6, 8, 3 months, 16 or any number up to 52), the days you can
   train, and the day of the long run. A race is on the long-run day of the last week.
3. **Preview**: what the plan starts from (km a week lately, runs a week, longest run, VO2max) and the plan week by
   week, with warnings, for example when the plan is short for the distance or there are too few runs to work out
   paces. Tick **Also set a goal** to create a matching goal on the [Goals](goals.md) page: the race distance in one run
   by the race day, or the plan's weekly km every week.

## How the plan is built
- **Phases**: a race plan goes from *Base* to *Build*, *Peak* and *Taper* (one week before 5 and 10 km, two before a
  half marathon or marathon). Shape and fitness plans have three building weeks and an *easier week*, again and again.
- **Volume**: the first week has about the km you ran per week in the last 8 weeks (at least 10 km). It grows by at most
  10 % a week, every fourth week is easier, and it stays under a peak that depends on the distance.
- **Long run**: a share of the week's km on the long-run day, growing by at most 2 km a week, up to 14 km for a 5 km
  race, 17 km for 10 km, 21 km for a half marathon and 32 km for a marathon.
- **Hard sessions**: one a week with up to three training days, two with four or more, never on the day before or after
  the long run or another hard session. Tempo runs (threshold pace) come first; intervals and race-pace work follow in
  the build and peak weeks.
- **Paces** come from your VO2max, estimated from your runs as on the Health data page or taken from your
  [profile](user-settings.md), after Jack Daniels' tables: easy, marathon, threshold, interval and repetition pace.

## Following the plan
The Training page shows the plan's weeks as rows of seven days, with the phases as a coloured strip, the km planned
and done per week, and a card per session. Click a session to change its kind, date, distance, pace, repetitions or
note, to skip it or to delete it. Hover a day and click **+** to add a session. Changed sessions are kept when you
**Adapt plan**: it rebuilds the sessions from today on from your runs of the last weeks, never with more km than the
plan had for this week.

The dashboard shows the sessions of this week and next week in a row below the goals. In the four-week calendar, days
with a session get a ring: dashed in the session's colour while ahead, in the colour of how it went once done or past.
Both move with the dashboard's date arrows.

## Matching runs to sessions
Every run is matched to a session automatically: a run on the session's day first, otherwise one on the day before or
after that no other session took. In the session window you can link another run from the days around it by hand, say
that no run belongs to the session, or go back to automatic matching. An activity that did a session shows a small
calendar mark in the dashboard feed and the activity panel.

Each session gets a verdict:

- **Done as planned**: the distance within 20 % and the pace in range.
- **Harder than planned**: an easy, recovery or long run clearly faster than easy pace, an easy run much longer than
  planned, or an average heart rate above zone 2 (once your heart-rate zones are saved).
- **Easier than planned**: the fastest kilometres of a tempo or interval run slower than its pace.
- **Partly done**: less than 80 % of the planned distance.
- **Missed**: a past session without a run that wasn't skipped.

Matching is worked out from your runs each time, so importing, editing or deleting a run updates it right away.

## AI coach
With AI turned on in the settings (a local Ollama model), the Training page can explain the current week, and the
session window can comment on how a run did its session. The plan doesn't need it: without a model these buttons are
just not shown.
