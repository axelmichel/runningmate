
# RunningMate
![Alt Text](images/logo-runningmate.png)
## Introduction
RunningMate is a desktop application for storing and analyzing sport activities, built with Rust and Tauri. 
In the current version it can handle running, cycling and walking activities. 
This is - and will be for a while - work in progress, so expect some bugs and missing features. 
But I try my best and I really "eat my own dogfood". I am using this for my own runs and rides, 
so the motivation is high to keep it running and add new features regulary.

## Why RunningMate?
I run and ride for over 25 years, I was never a pro, I never wanted to be one, but I always wanted to know how I am doing. 
A little data obsessed so to say. I like my stats, my charts and maps, not to compare myself with others, but with myself.

Over the years I used a lot of different tools and devices, from a handwritten running dairy to Excel sheets, from Garmin Connect to Strava, 
from Runtastic to NikeRun, I used it all. Most of the services are great, but they all have their limitations and flaws. 
From incorrect data to missing features, from privacy concerns to data export limitations. Not to mention the pricing. 

The social aspect of most of the available services is not my thing. Personally, I find some of the virtual communities rather toxic 
and not particularly motivating. On top of that, leaderboards are almost always flooded with absurd scores. Same for my own 
“personal bests” - data that might look impressive, but is far from reality.

So if you are a bit like me, and you even use(d) different devices like polar, garmin, suunto, wahoo, etc. you might find yourself in the
situation that you either have to pay to get more than a very basic history list, or you end up with a lot of raw data files or even worse, 
the data is gone for good. That is the reason why I started this project. 
A collection of tools to store, analyze and visualize your very own activity data.

And with that, happy running, riding, or walking! (*There might be more in the future*)

## Installation
Download the latest release from [GitHub](https://github.com/axelmichel/runningmate/releases) and open the installer for your system
(on macOS the `.dmg`). The app is not code-signed yet, so macOS asks for confirmation the first time you open it.

All data stays on your computer in the `RunningData` folder in your home directory: the database, the archived activity files and your photos and videos.
Activities come in through [**Sync**](ui/sync.md): TCX and GPX files, a watched folder (e.g. an iCloud export) or Garmin Connect, which also works in the other direction.

## What's in the app
- [Dashboard](ui/dashboard.md): latest photos, the last four weeks, goals, the training week, volume and an activity feed.
- [Activities](ui/main-window.md): lists per sport with search, filters and a panel with the key figures.
- [Detail view](ui/detail-window.md): maps, charts, splits, zones, training effect, an AI summary, a social-share image and
  editors to fix GPS, heart rate and steps.
- [Health data](ui/health.md), [Segments](ui/segments.md), [Goals](ui/goals.md), [Training plans](ui/training.md),
  the [Heat map](ui/heat-map.md) and the [Gallery](ui/gallery.md).
- [Settings](ui/settings.md) and [User settings](ui/user-settings.md).

How the figures are computed is explained under **Metrics**, starting with [how a file becomes an activity](metrics/activity-data.md).

### Building it yourself
You need [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 22 with pnpm, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your system.

```sh
cd app
pnpm install
pnpm tauri dev     # run in development mode
pnpm tauri build   # build the installer
```

## Contributing
Feel free to contribute by submitting issues or pull requests on [Github](https://github.com/axelmichel/runningmate). Make sure to follow best practices and include documentation for any new features.

## Copyrights and Licenses

### Icons
Icons from [Lucide](https://lucide.dev), licensed under the [ISC License](https://lucide.dev/license).

### Maps
The maps are drawn with [Leaflet](https://leafletjs.com/) on [Esri](https://www.esri.com) basemap tiles with data from [OpenStreetMap](https://www.openstreetmap.org) contributors.

### Charts
The charts are created with [Recharts](https://recharts.org) (MIT licensed).

### Place Names
Place names are looked up with [Nominatim](https://nominatim.openstreetmap.org/) from [OpenStreetMap](https://www.openstreetmap.org/copyright) data, © OpenStreetMap contributors, available under the [Open Database License](https://opendatacommons.org/licenses/odbl/).

### Weather Data
The weather data is fetched from [Open-Meteo](https://open-meteo.com/). API data are offered under Attribution 4.0 International (CC BY 4.0), for further details see [here](https://open-meteo.com/en/license).