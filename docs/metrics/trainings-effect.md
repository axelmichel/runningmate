# Trainings Effect

The Effect tab of the detail view rates what an activity did for your fitness in the way Garmin, Suunto and Polar
watches do with the method of Firstbeat: from **EPOC**, the excess post-exercise oxygen consumption, the extra oxygen
your body needs after exercise to return to rest. The harder and longer you train, the higher EPOC climbs; easy
stretches let it fall again.

From the heart rate of the activity RunningMate works out

- the **peak EPOC** in ml of oxygen per kg of body weight,
- the **aerobic training effect** from 0.0 to 5.0, from the peak EPOC and your fitness level,
- the **anaerobic training effect** from 0.0 to 5.0, from the time close to your maximum heart rate,
- the **recovery time** in hours.

Firstbeat's model is proprietary. Its published form (the white paper *EPOC Based Training Effect Assessment*) is
followed here, but the formulas and constants below are RunningMate's own approximation: the values come close to
what a watch shows, but they won't be the same.

Beside it, the Effect tab shows the [effect score](#the-effect-score-112) from 1 to 12, RunningMate's own rating carried
over from the first version of the app. It compares the activity with your training of the last 90 days instead.

## Data used
- The **heart rate** of every trackpoint of the archived activity file. Each point's heart rate counts for the time
  since the previous point. Without a recorded track, the [splits](activity-data.md#splits) are used instead: each
  split's average heart rate for its duration. Without any heart rate there is no training effect.
- Your **maximum** and **resting heart rate** from the [user settings](../ui/user-settings.md). When one is missing,
  a fallback is used (see [Fallbacks](#fallbacks)); the Effect tab says which.
- Your **VO2max** for the fitness level: estimated from your runs of the 90 days up to the activity, as on the
  [Health page](../ui/health.md), or the value from your profile when there are not enough runs.

## Intensity
The intensity is the share of the heart-rate reserve, the range between resting and maximum heart rate (as in the
Karvonen zones):

\( I = \frac{HR - HR_{rest}}{HR_{max} - HR_{rest}} \)

kept between 0 and 1. The share of the heart-rate reserve is close to the share of the oxygen uptake reserve, which
is why Firstbeat and others use it to measure the intensity from the heart rate.

## EPOC
EPOC is accumulated step by step: \( EPOC_t = f(EPOC_{t-1}, I_t, \Delta t) \). At a constant intensity it moves
exponentially towards a **steady state** that depends on the intensity: upwards when the intensity is higher than
what the current EPOC corresponds to, downwards (recovery) when it is lower.

- EPOC is generated at a rate that rises exponentially with the intensity (ml/kg/min):

    \( g(I) = 0.015 \times \left( e^{6.5\,I} - 1 \right) \)

- It approaches the steady state with a time constant that is short at rest and long when hard (minutes):

    \( \tau(I) = 10 + 50 \times I^3 \)

- The steady state is \( EPOC_{ss}(I) = g(I) \times \tau(I) \), and after \( \Delta t \) minutes at intensity \(I\)

    \( EPOC_t = EPOC_{ss}(I) + \left( EPOC_{t-1} - EPOC_{ss}(I) \right) \times e^{-\Delta t / \tau(I)} \)

EPOC starts at 0. The **peak EPOC** is the highest value during the activity. Because the time constant grows with
the intensity, EPOC keeps climbing during long hard efforts, while easy running levels off early: an hour at 65 % of
the reserve ends at about 22 ml/kg, 45 minutes at 82 % at about 81 ml/kg.

**Pauses.** As for the [activity data](activity-data.md#duration-and-pauses), a gap of more than 10 seconds between
two trackpoints is a pause. A pause, and any stretch without heart rate, counts as rest (\(I = 0\)): EPOC only decays
during it, so a pause never adds EPOC, whatever the heart rate after it.

## Aerobic training effect
The peak EPOC is compared with thresholds that grow with your fitness: a fitter athlete needs more EPOC for the same
effect. For an average VO2max of 45 ml/kg/min they are

| Aerobic effect | 1 | 2 | 3 | 4 | 5 |
|----------------|---|---|---|---|---|
| Peak EPOC (ml/kg) | 6 | 15 | 35 | 80 | 180 |

For your VO2max they are multiplied by the fitness factor

\( s = \frac{VO_2max}{45} \)

kept between 0.75 and 1.4; without a VO2max \( s = 1 \). Up to the first threshold the effect rises linearly from 0
to 1; between two thresholds \( T_k \) and \( T_{k+1} \) it is interpolated on a log scale:

\( TE = k + \frac{\ln(EPOC_{peak} / T_k)}{\ln(T_{k+1} / T_k)} \)

and from the last threshold on it is 5.0. The effect is shown with one decimal.

## Anaerobic training effect
Firstbeat's anaerobic training effect also looks at the speed or power of short, hard bursts. RunningMate only has
the heart rate, which reacts too slowly to see a sprint, so this is a rough **approximation** from the time spent
close to the maximum:

1. **Anaerobic load.** Every minute above 85 % of the reserve counts, weighted by the square of how far above:

    \( A = \sum \left( \frac{I - 0.85}{0.15} \right)^2 \times \Delta t \) (minutes)

2. **Repeated bursts.** A burst is a stretch of at least 15 seconds at 90 % of the reserve or more (zone 5 of the
   Karvonen zones). Every burst after the first raises the load by 25 %, up to +200 %, so intervals count more than the
   same time in one piece:

    \( b = 1 + \min\left(0.25 \times (n - 1),\ 2\right) \)

3. **Effect.** \( TE_{an} = 5 \times \left( 1 - e^{-A \times b / 15} \right) \), shown with one decimal.

An activity that never gets above 85 % of the reserve has an anaerobic effect of 0.0.

## The scale
Both effects use the usual wording:

| Effect  | Meaning |
|---------|---------|
| 0.0–0.9 | No effect |
| 1.0–1.9 | Minor |
| 2.0–2.9 | Maintaining |
| 3.0–3.9 | Improving |
| 4.0–4.9 | Highly improving |
| 5.0     | Overreaching |

## Recovery time
The recovery time grows with the peak EPOC relative to the threshold of aerobic effect 3:

\( h = 24 \times \left( \frac{EPOC_{peak}}{35 \times s} \right)^{0.75} \)

at most 96 hours. An activity at the threshold of effect 3 needs a day. Above 24 hours it is shown in whole days.

## Fallbacks
| Missing | Used instead |
|---------|--------------|
| Maximum heart rate | the upper bound of zone 5; else \( 220 - \text{age} \) from your birthday on the day of the activity; else 185 bpm |
| Resting heart rate | 60 bpm (also when the entered value is less than 20 bpm below the maximum) |
| VO2max | an average fitness level, \( s = 1 \) |
| Track | the split averages |

A maximum heart rate set too low makes every activity look harder, a resting heart rate set too high as well. The
Effect tab lists the heart rates and the VO2max it used and where they come from.

## Example
You run 50 minutes at a steady 150 bpm. Your profile has a resting heart rate of 55 and a maximum of 185, and your
runs give a VO2max of 50.

- Intensity: \( I = \frac{150 - 55}{185 - 55} \approx 0.73 \).
- Generation: \( g = 0.015 \times (e^{6.5 \times 0.73} - 1) \approx 1.72 \) ml/kg/min; time constant
  \( \tau = 10 + 50 \times 0.73^3 \approx 29.5 \) minutes; steady state \( 1.72 \times 29.5 \approx 50.7 \) ml/kg.
- Peak EPOC after 50 minutes: \( 50.7 \times (1 - e^{-50/29.5}) \approx 41.4 \) ml/kg.
- Fitness factor \( s = 50 / 45 \approx 1.11 \): the thresholds of effect 3 and 4 are about 38.9 and 88.9 ml/kg.
- Aerobic effect: \( 3 + \frac{\ln(41.4 / 38.9)}{\ln(88.9 / 38.9)} \approx 3.1 \), **Improving**.
- Anaerobic effect: the heart rate stays below 85 % of the reserve, so **0.0**, no effect.
- Recovery: \( 24 \times (41.4 / 38.9)^{0.75} \approx 25 \) hours, shown as 1 day.

Some more examples for an average fitness level (VO2max 45):

| Activity | Peak EPOC | Aerobic | Anaerobic | Recovery |
|----------|-----------|---------|-----------|----------|
| 30 min at 60 % | 12 ml/kg | 1.7 | 0.0 | 10 h |
| 60 min at 65 % | 22 ml/kg | 2.5 | 0.0 | 17 h |
| 120 min at 70 % | 38 ml/kg | 3.1 | 0.0 | 25 h |
| 45 min at 82 % | 81 ml/kg | 4.0 | 0.0 | 45 h |
| 5 × 5 min at 93 % with 3 min easy, warm-up and cool-down | 101 ml/kg | 4.3 | 3.1 | 53 h |
| 45 min at 92 % | 174 ml/kg | 5.0 | 2.4 | 80 h |

## Things to keep in mind
- The training effect is based on heart rate only. A heart rate that runs high from heat, caffeine, illness or a
  slipping chest strap raises it; one that lags behind short sprints lowers the anaerobic effect.
- The aerobic effect depends on your fitness level, so the same run rates lower as your VO2max rises.
- Your heart rate zones aren't used, except for the zone 5 bound as a fallback for the maximum heart rate; they are
  only used for the [time in zones](heart-rate-zones.md).
- The fitness, fatigue and form chart on the same tab is described under [Fitness and form](fitness.md).

## The effect score (1–12)
The effect score is a simpler rating that doesn't need your maximum or resting heart rate. It compares how hard the
activity was with how hard your activities of the same sport were in the 90 days before it, using the averages of the
splits.

**Load.** For runs and walks the load of a split is its average heart rate times its pace (min/km) over its speed
(m/s) plus one; for rides the power (W) takes the place of the pace:

\( L_{\text{run}} = \frac{HR \cdot \text{pace}}{v + 1} \qquad L_{\text{ride}} = \frac{HR \cdot P}{v + 1} \)

**Baseline.** The mean load of all splits of the same sport in the 90 days before the activity, at least 10 (10 without
any). Splits without heart rate, pace or speed (and, for rides, power) are left out, so a run recorded without a strap
doesn't pull the baseline down.

**Score.** The activity's load \( L \) from the averages of all its splits is weighted by how high its heart rate was and
compared with the baseline \( B \):

\( z = 1 + \frac{HR - 120}{20} \qquad E = \left( \frac{L \cdot z}{B + 1} \right)^{0.8} \qquad
\text{score} = \operatorname{round}(6E), \text{ between 1 and 12} \)

An activity missing any of the averages scores 1. The suggested rest is \( 6 \cdot e^{0.28 \cdot \text{score}} \) hours,
from about 8 hours for 1 to 5 days for 12.

| Score | Meaning |
|---|---|
| 1–2 | Recovery |
| 3–5 | Base endurance |
| 6–7 | Tempo |
| 8–9 | Threshold |
| 10–11 | VO2max |
| 12 | Anaerobic |

Because the score is relative to your own recent training, the same run scores lower after a block of hard weeks than
after a quiet month. Unlike the training effect, it doesn't know your heart-rate reserve or fitness, and as the load uses
pace, a slow run at a high heart rate scores high.

