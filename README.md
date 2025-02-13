# RunningMate

RunningMate is a Python-based application designed to process, analyze, and visualize activity data. The current release supports cycling, running and walking activity data in form of `tcx` files. The application is built using the [PyQt6](https://pypi.org/project/PyQt6/) library for the GUI, Pandas for data processing, and [folium](https://python-visualization.github.io/folium/latest/) for visualization.

The app does not store any of your data outside of your local machine. All collected data can be found in the `runningData` folder within your home directory. The imported `tcx` files are zipped, and the extracted data is stored in a SQLite database. Uploaded media files are stored in the `media` folder. Generated images like track maps or elevation charts are located in the `images` folder. In the current Version this app does not support multiple users.

## Features
- Map of each track.
- Elevation graph.
- Abstract map image of the track to share or use as overlay for images.
- Calculation and visualization of key metrics such as distance, pace, speed, heart rate, power,and elevation gain.


## Installation

### Prerequisites
Make sure you have Python 3.8 or later installed on your system. You can download it from [python.org](https://www.python.org/downloads/).

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
Run the tests with:

```sh 
pytest
```

#### Linting
Lint the code with:

```sh
black . && isort . &&  ruff check .
# or in case you want to fix automatically
black . && isort . &&  ruff format .
```


## Future Enhancements
- Integration of weather data
- Search and filter activities
- Calendar with activities
- Weekly, monthly, and yearly statistics with comparisons
- Heatmaps
- Integration of other activity data formats (e.g., GPX)
- Integration of other activity data sources (e.g., Garmin Connect)
- Heart-rate zone analysis
- Activity details (e.g., splits, laps)
- Tools to clean and repair data (e.g., remove outliers)


---

Happy Running! 🏃‍♂️🏃‍♀️
