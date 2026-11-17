# Pace

Pace is the time you need per kilometre (min/km), speed the distance per hour (km/h). RunningMate shows several of
both, and they aren't all computed the same way. This page explains which is which, so you know why, for example,
the average pace and the average speed of an activity don't always match.

All paces are the actual paces on the terrain you covered. For runs and walks RunningMate also computes a
[grade-adjusted pace](#grade-adjusted-pace-gap) (GAP) as a separate value: the pace the same effort would have given
on the flat. It never replaces the pace; hills also show up in the [calories and power](activity-data.md#power),
which count the metres you climbed.

## Elapsed time and pauses
The **duration** of an activity is the elapsed time from the first to the last trackpoint, pauses included. A
**pause** is any gap of more than 10 seconds between two trackpoints, which is what devices with auto-pause leave
when you stop. The **Pause** value on the detail view is the sum of those gaps. Waiting at a traffic light with the
device still recording every second is not a pause.

**Moving time** (duration minus pauses) is used for the cardio fitness estimate and the training plan, but not shown
for the activity itself.

## Pace per point
Every trackpoint gets a pace from the distance and the time to the point before it. Points with a pace outside the
plausible range for the activity type are left out: 3:00–12:00 min/km for runs, 0:30–6:00 min/km for rides and
8:00–25:00 min/km for walks. Where you stood still, the distance is filled in from the neighbouring points, and with
one-second recording the time gaps are smoothed over five points. The details are under
[Activity data](activity-data.md#speed-and-pace-per-point).

The pace chart on the Statistics tab shows these per-point values.

## Values of an activity

| Value | How it is computed | Pauses |
|-------|--------------------|--------|
| **Average speed** (km/h) | distance ÷ duration | included |
| **Average pace** (min/km) | mean of the per-point paces | mostly left out |
| **Fastest pace** | the 5th percentile of the per-point paces | left out |
| **Slowest pace** | the 95th percentile of the per-point paces | left out |
| **GAP** (min/km, runs and walks) | average pace × distance ÷ flat distance costing as much | left out |

**Average speed** is the plain distance over the elapsed time, so every pause and stop lowers it.

**Average pace** is the mean of the per-point paces. A pause is a single long gap between two points; it usually gives
one implausible pace that is dropped, or one value among hundreds. Standing still while recording gets the pace of
the neighbouring points. So the average pace is close to your pace while moving, and on an activity with stops it is
faster than the average speed suggests.

**Example:** you run 10 km in 55 minutes and wait 5 minutes at traffic lights with auto-pause. The duration is
1:00:00, so the average speed is 10.0 km/h, which is a pace of 6:00 min/km. The average pace is about 5:30 min/km,
your pace while moving.

The **fastest** and **slowest pace** are percentiles rather than the extremes, so a single GPS glitch doesn't set
them: 5 % of your points were faster than the fastest pace, 5 % slower than the slowest.

For rides the same values are computed; the activity list shows the average pace for rides too.

## Splits
Each split (1 km for runs and walks, 5 km for rides) has:

- a **time**: from its first to its last trackpoint, pauses included;
- a **pace**: that time divided by the split's distance, so a pause within the split makes it slower;
- a **speed**: the mean of the split's per-point speeds, which, like the average pace, hardly notices pauses;
- for runs and walks a **GAP**: the split's time over the flat distance that costs as much as the split (see
  [below](#gap-of-a-split)).

That is why a split's pace and speed don't always convert into each other exactly. See
[Activity data](activity-data.md#splits) for how the splits are cut.

## Grade-adjusted pace (GAP)
Running uphill costs more energy per metre than running on the flat, running gently downhill less, and running
steeply downhill more again, because you brake. The grade-adjusted pace is your pace converted to the flat pace that
would have cost the same energy per minute. It is computed for runs and walks, not for rides, and shown beside the
pace: in the activity panel, as a column on the Splits page and as a dashed line on the pace and speed charts.

### Energy cost of a gradient
RunningMate uses the energy cost of running and walking on gradients measured by Minetti et al. (2002, *Energy cost
of walking and running at extreme uphill and downhill slopes*, Journal of Applied Physiology 93: 1039–1046), in
J per kg and metre for the gradient \( i \) as a fraction (0.05 = 5 % uphill, −0.05 = 5 % downhill). For runs:

\( C_r(i) = 155.4\,i^5 - 30.4\,i^4 - 43.3\,i^3 + 46.3\,i^2 + 19.5\,i + 3.6 \)

and for walks, from the same paper:

\( C_w(i) = 280.5\,i^5 - 58.7\,i^4 - 76.8\,i^3 + 51.9\,i^2 + 19.6\,i + 2.5 \)

On the flat a run costs \( C_r(0) = 3.6 \) J/kg/m. The grade-adjusted speed is the speed times the cost on the
gradient over the cost on the flat, so the grade-adjusted pace is the pace divided by it:

\( v_{GAP} = v \times \frac{C(i)}{C(0)} \qquad p_{GAP} = p \times \frac{C(0)}{C(i)} \)

The cost is lowest at about −18 % for runs (−15 % for walks): there the same effort carries you fastest, so the GAP
is much slower than your pace. Steeper downhill the cost rises again, and from about −40 % (−32 % for walks) the GAP
is faster than your pace. Gradients steeper than ±45 %, the range Minetti measured, count as ±45 %.

### Gradient of a point
The elevation of GPS and even barometric recordings is too noisy to take a gradient from one point to the next: a
2 m jump over 15 m is already a 13 % gradient. So the gradient of a point is the slope of a straight line fitted
(least squares) to the elevation over 100 m of track around it, 50 m before and 50 m after, measured along the
track. Standing still doesn't change it, and a track shorter than 20 m counts as flat. Each point's grade-adjusted
pace, the dashed line on the charts, is its [pace](#pace-per-point) divided by the cost factor of its gradient. The
point after a pause (a gap of more than 10 seconds) gets none, and neither do points without a plausible pace.

### Average GAP of an activity
The GAP of an activity starts from its [average pace](#values-of-an-activity), so it treats pauses and stops the same
way, and adjusts it by the energy of the whole activity:

\( p_{GAP} = \bar{p} \times \frac{\sum d}{\sum d \times C(i)/C(0)} \)

with the average pace \( \bar{p} \) and the sums over the steps \( d \) between the trackpoints that have a
grade-adjusted pace. The second sum is the flat distance that would have cost as much as the activity. On a flat
activity the GAP is the average pace. On a hilly loop it is faster: a climb costs more than the same descent saves
(5 % up costs 1.301 times the flat, 5 % down 0.763 times, together more than twice the flat). A plain mean of the
per-point grade-adjusted paces would come out slower instead, because a pace divided by 1.301 gains less than one
divided by 0.763 loses, and it wouldn't match the splits.

### GAP of a split
A split's GAP is formed like its pace, the split's time over a distance: the flat distance that would have cost the
same energy, the sum of each step's distance times the cost factor of its gradient (a step over a pause counts as
flat). So a flat split's GAP is its pace, and a pause within the split makes it slower, as it does the pace.

**Example:** you run a kilometre at 6:00 min/km up a steady 5 % gradient. The cost is
\( C_r(0.05) = 4.685 \) J/kg/m, 1.301 times the 3.6 of the flat, so the GAP is \( 6{:}00 \div 1.301 = 4{:}37 \) min/km:
the effort of running 4:37 min/km on the flat. Down the same gradient the cost is 2.746 J/kg/m (0.763 times the
flat) and 6:00 min/km becomes a GAP of 7:52 min/km. Down a 45 % slope, braking costs 4.032 J/kg/m, 1.120 times the
flat, and 6:00 min/km is a GAP of 5:21 min/km.

## Best segments
The best segments are the fastest stretches of 1, 5, 10, 21 and 42 km for runs (other distances for rides and walks),
made of consecutive splits. Their pace is the total time of those splits over their total distance, pauses included.
Because they are built from whole splits, your best kilometre is your fastest 1 km split. Paces faster than the
plausible limit of the activity type (3:00 min/km for runs) are left out as recording errors. More under
[Activity data](activity-data.md#best-segments).
