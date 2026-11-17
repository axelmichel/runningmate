# Heart rate zones
Heart rate zones sort the intensity of your training by heart rate, from easy recovery to all-out efforts.
RunningMate uses five zones. In the user settings you choose how they are set: you enter them yourself, or they are
worked out from your maximum heart rate or from your heart-rate reserve. Until zones are set, the detail view and the
Health page show no time in zones.

## The zones
Each zone is given by its **upper bound** in beats per minute. Each zone starts one beat above the previous one.

| Zone | From | To |
|------|------|----|
| **Zone 1** | the start of zone 1 (see the methods) | zone 1 bound |
| **Zone 2** | zone 1 bound + 1 | zone 2 bound |
| **Zone 3** | zone 2 bound + 1 | zone 3 bound |
| **Zone 4** | zone 3 bound + 1 | zone 4 bound |
| **Zone 5** | zone 4 bound + 1 | zone 5 bound |

A heart rate counts in the first zone whose upper bound it doesn't exceed. A heart rate below the start of zone 1
still counts as zone 1, one above the zone 5 bound still counts as zone 5.

All analyses use the same zones: the time in zones, the colours of the heart-rate chart, the reference lines, the
check of easy runs in a training plan and the facts for the AI summary.

## Methods
### Custom
You enter the five upper bounds yourself, for example from a lab test or your watch. Each bound must be a whole number
between 40 and 240 bpm and higher than the one before. Zone 1 starts at half of the zone 5 bound.

Before you save bounds, the form suggests some at **60, 70, 80, 90 and 100 %** of your maximum heart rate, rounded
down:

- the maximum heart rate from your profile, if you have entered one;
- otherwise the age-predicted maximum \( HR_{max} = 220 - \text{age} \), with your age taken from your birthday.

**Example:** with a maximum heart rate of 185 bpm the suggestion is 111, 129, 148, 166 and 185 bpm. Zone 1 then runs
from 92 (half of 185, rounded down) to 111 bpm, zone 2 from 112 to 129 bpm, and so on. Without a maximum heart rate,
at 56 years the formula gives \( 220 - 56 = 164 \) bpm and the bounds 98, 114, 131, 147 and 164.

Custom bounds stay as they are when you change your profile.

### Percent of maximum heart rate
The zones end at **60, 70, 80, 90 and 100 %** of the maximum heart rate from your profile, and zone 1 starts at 50 %.
Each bound is \( p \times HR_{max} \), rounded down, with \( p \) = 0.6, 0.7, 0.8, 0.9 and 1.0 (0.5 for the start of
zone 1). These are the bounds of the suggestion above, but they follow your profile: a new maximum heart rate moves
the zones with it. The age formula isn't used here; the method needs a maximum heart rate in the profile.

**Example:** at 185 bpm the zones end at 111, 129, 148, 166 and 185 bpm and zone 1 starts at 92 bpm.

### Karvonen (heart-rate reserve)
The Karvonen method takes the same shares of your **heart-rate reserve**, the range between resting and maximum heart
rate, and adds them to the resting heart rate. Each bound is \( HR_{rest} + p \times (HR_{max} - HR_{rest}) \),
rounded down, with the same \( p \) as above. Since the resting heart rate is part of it, the zones sit higher than
with a plain percentage of the maximum, most of all in the lower zones. The method needs both heart rates in the
profile, and the resting one must be below the maximum.

**Example:** with a resting heart rate of 50 and a maximum of 185 bpm the reserve is 135 bpm. Zone 1 starts at
\( 50 + 0.5 \times 135 = 117 \) bpm and the zones end at 131, 144, 158, 171 and 185 bpm.

## Time in zones of an activity
The Zones tab of the detail view (and the heart-rate chart on the Effect tab for runs and rides) uses every trackpoint
of the activity's archived file. Each trackpoint's heart rate counts for the time since the trackpoint before it:

- gaps of more than **30 seconds** are left out, so a pause isn't counted at the heart rate you had when you started
  again;
- trackpoints without a heart rate (or with 0) are left out.

The percentages are shares of the time with a heart rate, not of the whole duration.

**Example:** your watch records every second, and a trackpoint at 142 bpm follows one a second earlier. With the
bounds above (zone 3 up to 148), that second counts in zone 3.

### Without the track file
If the archived file of an activity is missing, RunningMate falls back to the [splits](activity-data.md#splits):
each split counts its full time at its average heart rate. That is coarser: a kilometre that averaged 150 bpm counts
entirely in one zone, even if you spent half of it lower and half higher.

## On the Health page
The time in zones on the Health page always uses the splits method, over all activities of any type in the chosen
period. It is therefore a little coarser than the per-activity view.

## Reference lines
The heart-rate trend charts (detail view and Health page) draw your zone 1, zone 3 and zone 5 bounds as the
**Min**, **Ideal** and **Max** lines. The trend line is the mean of your last 30 activities' average heart rates
(on the detail view only activities of the same type in the 90 days up to the activity).
