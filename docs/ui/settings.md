# Settings

The Settings dialog holds the settings for the app itself. Your profile, heart-rate zones, shoes and bikes are in the
[user settings](user-settings.md). Open it with **Settings** at the bottom of the sidebar, with **RunningMate →
Settings…** or with ⌘,. Changes are saved as soon as you make them. They are stored in
`~/RunningData/appdata/settings.json`.

## General
- **Language**: English or Deutsch. The language is also used for the titles of newly imported activities. Français
  can be chosen, but the app is shown in English then.
- **Appearance**: **Follow the system**, **Light** or **Dark**.
- **Adding photos and videos from files**: what happens when you add a photo or video from disk to an activity.
  **Always copy into RunningMate** copies the file into `~/RunningData/media`. **Always link to the originals** keeps
  only a link to the file where it is, so it takes no extra space, but the photo is gone from RunningMate if you move or
  delete the file. **Ask each time** asks when you add files, with **Remember my choice (can be changed in the
  settings)** to set this setting from there. Photos from the Photos library are always copied.
- **Look up place names**: names the town and part of town each activity started in (for example "Freiburg im
  Breisgau, Wiehre"), shown in the activity panel, the detail window and the **Location** column, and found by the
  search. On by default. It sends the coordinates of the start, and nothing else, to
  [nominatim.openstreetmap.org](https://nominatim.openstreetmap.org), OpenStreetMap's place-name service: one activity
  a second at most, each only once (activities starting close to one already named reuse its name without asking).
  New activities are looked up after the import, older ones in the background. Turn it off to send nothing; names
  already found are kept. Names are in the language set when they were looked up.

## Activity panel
Choose which boxes the panel beside the activity list and the detail window show, separately for **Runs**, **Rides**
and **Walks**. Tick or untick a box per sport:

- **Route map**
- **Garmin Connect link**
- **Goals, sessions and trophies**
- **Photos, comment and summary** (only beside the activity list)
- **Weather**
- **Best segments**

Other activities always show every box.

## AI summary
RunningMate can write a short summary of each activity with a language model that runs on your own computer, through
[Ollama](https://ollama.com). It is off until you turn it on.

1. Install Ollama and a model, for example with `ollama pull qwen2.5:7b`.
2. Tick **Write activity summaries with a local AI model**.
3. **Ollama server** is `http://localhost:11434` by default. Change it only if Ollama runs elsewhere.
4. Click **Load models**. If none are installed, you are told so.
5. Pick a **Model**. Models with 7 to 14 billion parameters (for example `qwen2.5:14b`) work well.
6. **Compare with the last** sets how many weeks before the activity it is compared with: 2, 4 (the default), 8 or 12
   weeks.

Nothing is sent to an online service: the facts go only to the Ollama server you entered, which with the default
address is your own computer.

### The summary card
With summaries on and a model chosen, the **General** page of the [detail window](detail-window.md) shows a
**Summary** card, and the activity panel shows its start under **Photos, comment and summary**.

New activities from the last 14 days get a summary after they are imported. For other activities click **Write
summary**; **Rewrite** writes a new one. Re-importing an activity drops its summary, since it was written from the
old values; an activity from the last 14 days gets a new one.

RunningMate works out the comparison itself and gives the model only the results; the model is told not to calculate
anything. The facts are:

- the activity: sport, title, your comment, date, distance, time, pace or speed, heart rate, climb and temperature
- how it compares with your last 10 activities over a similar distance (within 10 %, at least 0.5 km)
- how it compares with your activities in the chosen weeks before it
- where available: pacing over the splits, training effect, time in the heart-rate zones, your training form and
  personal bests set in the activity

The summary is 4 to 6 sentences in your language, says what stands out and ends with a hint for the next training.
Below it you see which model wrote it. AI can be wrong.

The same setting turns on the AI notes on the [Training](training.md) page.

## Storage
Shows the **Data folder** (`RunningData` in your home directory), how many **Photos and videos** are copied there and
how much space they take, and the **Total size** of the folder. All data stays on this computer. The folder is shared
with the earlier Python version of RunningMate. See [Sync](sync.md#where-things-are-kept) for what is kept where.
