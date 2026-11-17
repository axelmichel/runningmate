# Fitness and form

The Health page and the Effect tab of the detail view show how your training adds up over weeks: **fitness**,
**fatigue** and **form** from a training-load model, and a **cardio fitness** estimate (VO2max) from your runs.

## Fitness, fatigue and form
RunningMate uses a training-load model in the style of Banister's fitness-fatigue model, as TrainingPeaks and
intervals.icu show it (CTL, ATL and TSB):

- **Fitness** is a slow average of your training load, over about 42 training days.
- **Fatigue** is a fast average, over about 7 training days.
- **Form** is fitness minus fatigue. After hard weeks fatigue is above fitness and form is negative; after easy days
  it turns positive.

### Load of an activity
Every [split](activity-data.md#splits) with a heart rate and a pace has a load

\( L = \frac{HR \times p}{v + 1} \)

with the average heart rate \(HR\) (bpm), the pace \(p\) (min/km) and the speed \(v\) (m/s) of the split, also for
rides. (The [training effect](trainings-effect.md) uses its own model, based on EPOC.) An activity's load is the mean of its
splits' loads, weighted by the activity type:

| Activity | Weight |
|----------|--------|
| Run      | 1.0 |
| Ride     | 0.8 |
| Walk     | 0.5 |

The loads of all activities on the same day (by UTC date) are added up to the day's load. Splits without heart rate
have no load, and an activity recorded without heart rate isn't a training day at all, so it doesn't pull the curve
down.

### The curve
The curve starts at your first activity with heart rate, with fitness and fatigue both set to that day's load. On
each later training day, with that day's load \(L\):

\( \text{fitness} = \text{fitness} + \frac{L - \text{fitness}}{42} \)

\( \text{fatigue} = \text{fatigue} + \frac{L - \text{fatigue}}{7} \)

\( \text{form} = \text{fitness} - \text{fatigue} \)

Unlike TrainingPeaks, the averages only move on days **with** training: a rest day doesn't lower fitness or fatigue,
the next activity does. Form includes the day's own training.

The curve is always computed from your first activity on, also when the chart only shows a part of it, so a day has
the same values in every period you look at.

**Example:** fitness and fatigue both stand at 100. Your next training day has a load of 200:

- fitness: \( 100 + \frac{200 - 100}{42} \approx 102.4 \)
- fatigue: \( 100 + \frac{200 - 100}{7} \approx 114.3 \)
- form: \( 102.4 - 114.3 = -11.9 \)

### On the Effect tab
The detail view shows fitness, fatigue and form for the 90 days up to the activity, as raw values.

### On the Health page
The load numbers have no unit anyone knows, so the Health page shows them as shares:

- **Fitness** is your fitness on the last training day of the period as a share of the highest fitness you ever
  reached up to the end of the period, your personal best. Below it is the change over the period, from the first to
  the last training day in it: \( \frac{\text{last} - \text{first}}{\text{first}} \times 100 \) %.
- **Form** is form as a share of fitness, \( \frac{\text{form}}{\text{fitness}} \times 100 \) %, rated in Joe Friel's
  zones as intervals.icu shows them:

| Form | Zone | Meaning |
|------|------|---------|
| below −30 % | High risk | Fatigue is far above your fitness; too much of this risks overtraining. |
| −30 to −10 % | Optimal | A productive training load: your fitness is building. |
| −10 to +5 % | Grey zone | Maintaining: not much building, not much recovering. |
| +5 to +20 % | Fresh | Rested and fit: a good time for a race or a test. |
| above +20 % | Transition | Very rested: without training your fitness starts to drop. |

A value on a boundary belongs to the zone above it (−10 % is the grey zone). In the example above, form is
\( \frac{-11.9}{102.4} \approx -11.6 \) % of fitness: **Optimal**.

## Cardio fitness (VO2max)
VO2max is the most oxygen your body can use, in ml per kg of body weight per minute. RunningMate has no lab or watch
measurement, so it estimates it from your runs with **Jack Daniels' and Jimmy Gilbert's VDOT formula**: the oxygen
cost of the pace you ran, divided by the share of VO2max you can hold for that long.

For a run of \(d\) metres in \(t\) minutes of moving time (duration minus pauses), with the velocity
\( v = d / t \) in metres per minute:

\( \text{cost} = -4.60 + 0.182258\,v + 0.000104\,v^2 \)

\( \text{share} = 0.8 + 0.1894393\,e^{-0.012778\,t} + 0.2989558\,e^{-0.1932605\,t} \)

\( VO_2max = \frac{\text{cost}}{\text{share}} \)

- Only runs of at least **3 km** in the chosen period count.
- Results below 15 or above 85 are treated as recording errors and dropped.
- At least **3 runs** are needed. Training runs aren't all-out races, so the estimate is the **90th percentile** of the
  runs' values: it reflects your harder runs without trusting a single outlier (for example a run with a wrongly
  recorded pause).
- With fewer runs, the VO2max from your user settings is shown instead, if you entered one.

**Example:** 5 km in 25:00 gives \( v = 200 \) m/min, a cost of about 36.0 and a share of about 0.94, so a VO2max of
about **38.3**.

### Rating
With your birthday and sex in the user settings, the estimate is compared with people of the same sex and age group
(20–29, 30–39 … 70–79; younger or older ages use the nearest group), using the Cooper Institute norms that Garmin
uses. The age is your age at the end of the period. The levels start at these values (ml/kg/min):

| Age | Men: Fair | Good | Excellent | Superior | Women: Fair | Good | Excellent | Superior |
|-----|-----------|------|-----------|----------|-------------|------|-----------|----------|
| 20–29 | 41.7 | 45.4 | 51.1 | 55.4 | 36.1 | 39.5 | 43.9 | 49.6 |
| 30–39 | 40.5 | 44.0 | 48.3 | 54.0 | 34.4 | 37.8 | 42.4 | 47.4 |
| 40–49 | 38.5 | 42.4 | 46.4 | 52.5 | 33.0 | 36.3 | 39.7 | 45.3 |
| 50–59 | 35.6 | 39.2 | 43.4 | 48.9 | 30.1 | 33.0 | 36.7 | 41.1 |
| 60–69 | 32.3 | 35.5 | 39.5 | 45.7 | 27.5 | 30.0 | 33.0 | 37.8 |
| 70–79 | 29.4 | 32.3 | 36.7 | 42.1 | 25.9 | 28.1 | 30.9 | 36.7 |

Below the Fair value the level is **Poor**. In the example, 38.3 is **Good** for a woman of 45 and **Fair** for a man
of 55.
