
# RunningMate
![Alt Text](images/logo-runningmate.png)
## Introduction
RunningMate is an application for storing and analyzing sport activities written in Python. 
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
The plan is to make this an executable application which can be installed on your system, but for now it's just a script.

### Prerequisites
For the moment you'll require python 3.13.2 or later to run the application. You can download it from [python.org](https://www.python.org/downloads/). 

### Install Dependencies
Extract the downloaded zip-file to a directory of your choice and navigate to it in your terminal. From the project directory install the required dependencies using:

   ```sh
   pip install -r requirements.txt
   ```

*Keep in mind, that you might have to rerun this command if you update to a newer version of the application.*

### Run the application

   ```sh
   python main.py
   ```

## Copyrights and Licenses

### Icons
All Icons made by [Remix](https://remixicon.com/). RemixIcon is licensed based on the [Apache License](https://github.com/Remix-Design/remixicon/blob/master/License) and all rights of products are reserved for RemixIcon.

### Maps
The maps are created with [Folium](https://python-visualization.github.io/folium/latest/index.html), [OpenStreetMap](https://www.openstreetmap.org) and [Leaflet](https://leafletjs.com/).

### Charts
The charts are created with [Plotly](https://github.com/plotly/plotly.py). `plotly.py` is [MIT](https://github.com/plotly/plotly.py/blob/main/LICENSE.txt) Licensed.

### Weather Data
The weather data is fetched from [Open-Meteo](https://open-meteo.com/). API data are offered under Attribution 4.0 International (CC BY 4.0), for further details see [here](https://open-meteo.com/en/license).