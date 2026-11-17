# Sync

The Sync dialog is where activities come in, and where they go out to Garmin Connect. Open it with **Sync** in the
sidebar, with **File → Sync…** or with ⌘O. It opens on the source you used last.

On the left you pick the source:

- **Garmin Connect**: imports new Garmin activities and uploads activities from elsewhere.
- **Folder sync**: imports new TCX and GPX files from a folder.
- **TCX / GPX files**: imports files you pick.

Strava and other services aren't supported yet.

## TCX / GPX files
Click **Choose TCX or GPX files…** and pick one or more `.tcx` or `.gpx` files. The files are copied into
RunningMate's archive; the originals stay where they are.

## Folder sync
Use this for a folder that another app exports to, for example an iCloud folder of RunGap or HealthFit. Click
**Choose…** to pick the folder. Below it you see **Last sync** and how many new files are in the folder: TCX and GPX
files changed since the last sync (on the first sync, all of them). Only the folder itself is read, not its subfolders.
Click **Sync** (the button shows the number of files) to import them.

Files in iCloud Drive may have to be downloaded first. While that happens the progress line shows
**Reading files… 3 of 40** with the current file name.

## Garmin Connect
Sign in with your Garmin **E-mail address** and **Password**. If your account uses two-step verification, Garmin sends
you a code; enter it under **Verification code** and click **Confirm**. Your password only goes to Garmin. RunningMate
keeps a sign-in token in the system keychain, not the password. If Garmin stops accepting the token, the sign-in form
comes back. **Sign out** forgets the token.

**Import from Garmin Connect** shows when the last sync ran and on which day the next one starts. **Sync now** downloads
the activities from that day on and imports them. The first sync fetches your whole history. Activities that are
already in RunningMate from another source (same day, about the same distance) are skipped before they are downloaded.

**Upload to Garmin Connect** sends activities that didn't come from Garmin, for example runs recorded with another
app. Set **Activities since** (30 days ago at first) and click **Upload**. The button shows how many
activities will go up: those since that date that weren't imported from Garmin and weren't uploaded before. Garmin skips activities it
already has.

## Limiting the period
Every source has optional **From** and **To** dates. With dates set, only activities in that period are imported;
**Any time** clears them. Without dates:

- Folder sync imports every new file.
- Garmin Connect imports everything since the last sync.

After a folder sync or Garmin sync with dates, the next sync still starts where the last complete one ended, so
nothing in between is skipped. The same holds when a sync is cancelled or a file fails: the folder sync time only
advances after a complete sync without dates or failures, and the next Garmin sync starts no later than the oldest
failed download.

In all cases the newest activities are imported first, so you see this week's runs right away while older ones follow.

## Progress
A bar and a list show each file as it is done:

- imported
- **Already imported**: a file with the same name was imported before. A run exported as both TCX and GPX counts as
  the same file.
- **Already there (same day and distance)**: an activity on the same day with about the same distance (within 1 %,
  at least 50 m) exists, for example the same run recorded by your watch and your phone. The file is not imported.
- **No GPS: draw its route under Waiting for a route** (see below)
- failed, with the reason

At the end you get a summary such as "12 imported · 3 already imported · 0 failed". **Cancel** stops after the current
file; what was imported so far stays. While a sync runs, the dialog can't be closed and you can't switch sources.

## Files without GPS
A file with times but no GPS positions, such as a treadmill run or a watch with GPS off, isn't imported straight away.
It is listed under **Waiting for a route** at the bottom of the Sync dialog, with its date, sport, duration and the
distance the device measured. Click **Draw route**, then click the route you ran on the map, from start to finish, and
save. Only then does it become an activity, with the same duplicate check as above. The bin icon removes a file from the
list. Drawing on the map works as in the route fix editor of the [detail window](detail-window.md).

## After the import
Each new activity is computed from its file: distance, pace, heart rate, splits, best efforts, segments and so on. See
[Activity data](../metrics/activity-data.md) for how. Then, without holding up the import:

- The weather of the day is fetched from [Open-Meteo](https://open-meteo.com) for the place halfway along the route. This needs
  an internet connection.
- The name of the place the activity started in (town and part of town) is looked up at
  [Nominatim](https://nominatim.openstreetmap.org), unless turned off in the [settings](settings.md#general). Lookups run
  one after another, at most one a second, so after a large import the names fill in over a while; activities imported
  earlier are caught up on the same way. Re-importing or fixing an activity looks the name up again if its start moved.
- If AI summaries are turned on in the [settings](settings.md#ai-summary), a summary is written for activities from the
  last 14 days. Older ones, for example when you import your whole history, get one only when you ask for it.

Goals, segments and training sessions pick up the new activities right away.

## Where things are kept
Everything lives in the `RunningData` folder in your home directory:

| Path | What it holds |
|---|---|
| `running_data.db` | The database with all activities and computed values. |
| `imports/` | One archive per activity, `{name}.tcx.tar.gz`, holding the TCX file. A GPX file is converted to TCX for this; the original GPX is kept in the same archive. |
| `imports/{name}.orig.tcx.tar.gz` | The original file of an activity you fixed in the detail window, kept so you can reset to it. |
| `imports/pending/` | Files waiting for a route. |
| `media/` | Photos and videos copied into RunningMate, photos scaled down to the [photo size](settings.md#photo-size). |
| `appdata/settings.json` | The [settings](settings.md). |
| `appdata/sync.json` | The sync folder and the last folder and Garmin sync. |

The archives are what RunningMate goes back to when an activity is recomputed, so don't delete the `imports` folder.
