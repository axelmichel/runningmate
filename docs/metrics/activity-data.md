# Activity data

This page explains how a file you import becomes an activity: which values RunningMate reads from the file, which it
computes, and with which formulas. Everything here happens once on import and again whenever an activity is
recomputed (after a route fix, a reset to the original, or one of the data fixes described at the end).

## Reading the file
RunningMate reads **TCX** and **GPX** files.

- **TCX**: every `<Trackpoint>` with a time is read, with its position, altitude, heart rate, cadence (`RunCadence`)
  and power (`Watts`). Calculations only use trackpoints that have a position *and* an altitude. Points without
  them stay in the file, for example as anchors for the route fix editor.
- **GPX** (1.0 and 1.1, e.g. from Garmin, Strava or Komoot): every `<trkpt>` with a time is read from all tracks. A
  point without `<ele>` takes the altitude of the point before it (0 m before the first). Heart rate and cadence come
  from Garmin's track point extension (`hr`, `cad`), power from a `power` element. The GPX is converted to TCX and
  archived as TCX, with the original GPX kept next to it. A GPX without times is a planned route, not a recording,
  and is rejected.

The sport comes from the TCX `Sport` attribute, or from the GPX `<type>`, which apps fill freely: anything containing
"run", "jog" or "lauf" becomes a run ("trail" a trail run), "cycl", "bik", "ride", "rad" or "velo" a ride, "walk",
"hik", "wander" or "trek" a walk. Strava's old numbers work too (1 ride, 9 run, 10 walk, 11 hike). A GPX without a
type counts as a run. A TCX with the sport "Other" counts as a walk.

A file that has times but no GPS at all (a treadmill run, a watch with GPS switched off) isn't imported straight
away. It waits until you draw its route in the route fix editor.

### Duplicates
A file is skipped if a file with the same name (without `.tcx` or `.gpx`) was imported before, so a run exported in
both formats is only imported once. It is also skipped if an activity on the same day has about the same distance:
within 1 %, and at least within 50 m. That catches the same run recorded by two devices or apps.

### Clock jumps
Some devices write wrong times, for example a run whose first trackpoints are dated correctly and all later ones two
weeks too late. A gap of more than **12 hours** between two trackpoints is treated as such a clock jump, not a pause:
every time after it moves back so that it follows the point before by one second, keeping the spacing of the times
after it. The activity then starts on the right day.

## Duration and pauses
The **duration** is the elapsed time from the first to the last trackpoint, pauses included.

A **pause** is any gap of more than **10 seconds** between two consecutive trackpoints. Devices with auto-pause stop
writing trackpoints while you stand still, which leaves such gaps. The activity's pause time is the sum of all of them.
Standing still while the device keeps recording every second is not a pause.

**Moving time** is the duration minus the pause time. It isn't shown on the activity itself, but the cardio fitness
estimate (see [Fitness and form](fitness.md)) and the training plan use it.

## Distance
The distance adds up the straight lines between consecutive trackpoints. For the activity's total the positions are
first projected onto a flat map (UTM, in the zone of the track's mean longitude); the splits and the per-point speed
use the distance on the earth's ellipsoid (WGS84) instead. Over an activity both agree to within a few metres.

## Elevation gain
The elevation gain is the sum of all rises between consecutive trackpoints, in metres. Descents don't subtract
anything. The altitudes are used as recorded, without smoothing, so a noisy barometer or GPS altitude adds a little
gain.

## Speed and pace per point
For every trackpoint RunningMate computes a speed and a pace from the distance and time to the point before:

- When more than 80 % of the gaps between trackpoints are exactly one second, the time gaps are smoothed over five
  points first, so speed and pace don't jump with every GPS fix. Pauses, steps and calories always use the real
  gaps.
- Where you stood still (no distance between two points), the distance is filled in from the neighbouring points, so
  a stop doesn't produce an extreme pace.
- A pace outside the range a person can manage for the activity type is left out:

| Activity | Pace kept | Minimum step |
|----------|-----------|--------------|
| Run      | 3:00–12:00 min/km | 0.5 m |
| Ride     | 0:30–6:00 min/km (10–120 km/h) | 5 m |
| Walk     | 8:00–25:00 min/km | 0.2 m |

Steps shorter than the minimum get no pace. How these values add up to the averages you see is described under
[Pace](pace.md).

## Power
If the file has power (a power meter or a running power pod), the measured watts are used. Otherwise RunningMate
estimates them from the speed \(v\) (m/s) and the climb rate \(c\) (metres climbed per second) of each point, using
your body weight \(m\) from the user settings (70 kg if none is set).

**Runs and walks** use the energy above resting from the ACSM metabolic equations (see Calories below), a quarter of
which becomes mechanical power:

\( P = m \times (h \times v + k \times c) \times 0.005 \times 4184 \times 0.25 \)

with \(h = 0.2\), \(k = 0.9\) for running and \(h = 0.1\), \(k = 1.8\) for walking. That gives roughly 1 W per kg
per m/s, which is what running power meters show. A 68 kg runner at 2.36 m/s (6:47 min/km) on the flat comes out at
about 168 W.

**Rides** use rolling resistance and air drag, with an 8 kg bike, a rolling resistance coefficient of 0.004, air
density 1.225 kg/m³ and a drag area of 0.27 m²:

\( P = 0.004 \times (m + 8) \times 9.81 \times v + \frac{1}{2} \times 1.225 \times 0.27 \times v^3 \)

The ride estimate has no climbing term, so it underestimates the power on a climb. The average power of the activity
is the mean of the per-point values.

## Calories
**Runs and walks** use the ACSM metabolic equations (American College of Sports Medicine). They give the oxygen you
use, in ml per kg per minute, from the speed and the grade:

\( VO_2 = 3.5 + h \times \text{speed} + k \times \text{speed} \times \text{grade} \)

with the same \(h\) and \(k\) as above (speed in metres per minute). Summed over the trackpoints, the speed term
becomes the distance covered and the grade term the metres climbed. Downhill counts as flat. Per point:

\( \text{kcal} = m \times (h \times \text{metres} + k \times \text{metres climbed} + 3.5 \times \text{minutes}) \times 0.005 \)

using 5 kcal per litre of oxygen. Standing still only adds the resting term, and measured watts don't change what a
run costs.

**Example:** 68 kg, 1 km on the flat in 6 minutes: \( 68 \times (0.2 \times 1000 + 3.5 \times 6) \times 0.005 \approx
75 \) kcal.

**Rides** use the power (measured or estimated), assuming 25 % of the energy you burn reaches the pedals:

\( \text{kcal} = \frac{P \times \text{seconds}}{0.25 \times 4184} \)

A gap of more than 10 seconds (a pause) adds nothing. At a steady 150 W, an hour comes to about 516 kcal.

## Heart rate
The average heart rate is the mean over all trackpoints that have one. Time in heart-rate zones is described under
[Heart rate zones](heart-rate-zones.md).

## Steps and cadence
For runs and walks, TCX cadence counts one foot, so RunningMate doubles it to steps per minute.

- **Cadence** (spm) is the mean of the doubled cadence over the trackpoints that have one.
- **Steps** adds up cadence × 2 × the time to the previous point. Gaps of more than 10 seconds (pauses) are left out,
  so a pause isn't walked through at the cadence of the point after it.

Rides have no steps.

## Splits
The activity is cut into splits of **1 km** (runs and walks) or **5 km** (rides). A split ends at the first trackpoint
where its own distance reaches the length, so a full split is never shorter than 1 km or 5 km (it can be a few metres
longer). The next split starts at that trackpoint. Whatever is left at the end becomes a shorter last split.

For each split RunningMate stores:

- the **time**, from its first to its last trackpoint, pauses included;
- the **pace**: that time divided by the split's distance;
- the **speed**, heart rate, power and cadence as the mean of the split's trackpoints;
- the **elevation gain**, the sum of the rises within the split;
- the position of its first trackpoint.

Splits are also the basis of the fitness curve, and of the training effect and the time in zones without a track file.

## Best segments
For every activity RunningMate looks for the fastest stretch over fixed distances:

| Activity | Distances (km) |
|----------|----------------|
| Run      | 1, 5, 10, 21, 42 |
| Walk     | 1, 5, 10, 15, 20, 30, 50 |
| Ride     | 5, 25, 50, 75, 100, 150, 200 |

A stretch is a run of consecutive splits, starting at any split and ending as soon as the splits cover the distance.
Its pace is the splits' total time over their total distance, so a short last split counts by its length. Because
stretches are made of whole splits, the best 1 km of a run is its fastest 1 km split, not the fastest kilometre
anywhere on the track. A stretch can't run over a split without time (a clock that ran backwards).

Records nobody can run are left out of the lists: a best segment faster than the fastest plausible pace of its
activity type (3:00 min/km for runs, 0:30 min/km for rides, 8:00 min/km for walks) isn't shown. That keeps out, for
example, a ride accidentally recorded as a run.

## Weather
After the import, RunningMate looks up the weather of the day at the start of the middle split from
[Open-Meteo](https://open-meteo.com): the day's maximum and minimum temperature, precipitation, maximum wind speed and
the weather code (sun, clouds, rain …). These are daily values, not the weather at the hour of the activity. Dates
within the last 60 days use Open-Meteo's forecast service (which covers recent days), older dates its historical
archive. The lookup is tried three times; if it still fails, or the day has no data yet, the activity has no weather.

## Data fixes at startup
Some values were computed wrongly by earlier versions. On startup RunningMate corrects stored activities once per
database, recomputing them from their archived files:

1. **Clock jumps**: activities whose file has a clock jump are imported again, so they move to the right day. A title
   generated from the wrong start is generated again, the AI summary is deleted and the weather is looked up again.
   Activities you have edited are left alone.
2. **Calories and power** are recomputed with the ACSM estimates described above.
3. **Pauses and splits**: calories, total steps, the splits and the best segments are recomputed with the pause
   handling and split cuts described above.
4. **Best segments** are recomputed so that none spans a split whose clock ran backwards.

Fixes 2 to 4 only change the values named; your title, comment, distance, duration and other edits stay as they are.
Activities whose archived file is missing are skipped.
