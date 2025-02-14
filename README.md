# RunningMate

![GitHub release](https://img.shields.io/github/v/release/axelmichel/runningmate?include_prereleases)

RunningMate is a Python-based application designed to process, analyze, and visualize activity data. The current release supports cycling, running and walking activity data. The application is built using the [PyQt6](https://pypi.org/project/PyQt6/) library for the GUI, Pandas for data processing, and [folium](https://python-visualization.github.io/folium/latest/) for visualization.

The app does not store any of your data outside of your local machine. All collected data can be found in the `runningData` folder within your home directory. The app uses a [SQLite database](https://www.sqlite.org/) for imported activity data.
Uploaded media files and generated images are stored in the `media` and generated images/charts in the `images` folder. In the current Version this app does not support multiple users.

## Documentation
The documentation can be found [here](https://axelmichel.github.io/runningmate).

## Features
- Import activity data from `tcx` files.
- Import activity data from garmin connect.
- Map of each track.
- Elevation graph.
- Abstract map image of the track to share or use as overlay for images.
- Calculation and visualization of key metrics such as distance, pace, speed, heart rate, power,and elevation gain.

### File Import
The app supports the import of `tcx` files. The files are stored in the `runningData` folder within your home directory. The imported `tcx` files are zipped, and the extracted data is stored in a SQLite database.


### Garmin Connect Synchronization
To sync your garmin activity data, you need to provide the username and password of your garmin account. The app uses the [garminconnect](https://pypi.org/project/garminconnect/) library to fetch the data. 
The data itself is stored in the same way as the imported `tcx` files. Your credentials are stored via [keyring](https://pypi.org/project/keyring/). 
The keyring library interfaces with the operating system's credential storage mechanisms, such as Keychain on macOS, Windows Credential Locker on Windows.

Be aware that the initial sync can take a while, depending on the number of activities you have. The app will only fetch new activities, so the sync time will decrease over time.
## Installation

### Prerequisites
Make sure you have Python 3.13.2 or later installed on your system. You can download it from [python.org](https://www.python.org/downloads/).

### Install Dependencies
Navigate to the project directory and install the required dependencies using:

```sh
pip install -r requirements.txt
```

## Usage

Run the application with:
   ```sh
   python main.py
   ```

## Contributing
Feel free to contribute by submitting issues or pull requests on GitHub. Make sure to follow best practices and include documentation for any new features.

### Development
```sh
pip install -r requirements.txt -r dev-requirements.txt
```

#### Debugging
Copy the env.example file to .env and adjust the values to your needs. Set `DEBUG_MODE` to True to enable logging and debugging features.

#### Testing
All pull requests will run the tests. Run the tests locally with:

```sh 
pytest
```

#### Linting
All pull requests will be linted. Run it upfront with:

```sh
black . && isort . &&  ruff check .
# or in case you want to fix what is fixable
black . && isort . &&  ruff format .
```
### Documentation
The documentation is written in markdown and can be found in the `docs` folder. The documentation is built using [MkDocs](https://www.mkdocs.org/). To build and run the documentation you need to install the following dependencies:
```sh
pip install mkdocs mkdocs-material https://github.com/mitya57/python-markdown-math/archive/master.zip  
```
To build the documentation run:
```sh
mkdocs build
```
To run the documentation locally run:
```sh
mkdocs serve
```

## Future Enhancements
- Integration of weather data
- Search and filter activities
- Calendar with activities
- Weekly, monthly, and yearly statistics with comparisons
- Heatmaps
- Integration of other activity data formats (e.g. GPX)
- Integration of other activity data sources (e.g. Strava)
- Heart-rate zone analysis
- Activity details (e.g., splits, laps)
- Tools to clean and repair data (e.g., remove outliers)


---

Happy Running! 🏃‍♂️🏃‍♀️
