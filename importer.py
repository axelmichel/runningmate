import os

from processing.compute_statistics import compute_run_db_data
from processing.parse_tcx import parse_tcx
from processing.data_processing import convert_to_utm, calculate_distance, calculate_pace, detect_pauses, calculate_steps
from processing.visualization import plot_track, plot_elevation, plot_activity_map
from processing.database_handler import insert_run, initialize_database

# Set paths
TCX_DIR = os.path.expanduser("~/RunningData/tcx")  # Directory containing TCX files
IMG_DIR = os.path.expanduser("~/RunningData/images")  # Directory for generated images

# Ensure directories exist
os.makedirs(IMG_DIR, exist_ok=True)

# Initialize the database
initialize_database()

def bulk_upload_tcx():
    """Processes all TCX files in the directory and inserts data into the database."""
    tcx_files = [f for f in os.listdir(TCX_DIR) if f.endswith(".tcx")]
    if not tcx_files:
        print("No TCX files found for processing.")
        return

    for file in tcx_files:
        file_path = os.path.join(TCX_DIR, file)
        base_name = os.path.splitext(file)[0]  # Remove extension
        date, time = base_name.split("_")[:2]  # Extract date and start time


        print(f"Processing {file}...")

        # Parse TCX file
        df, activity_type = parse_tcx(file_path)

        # Process data
        df = convert_to_utm(df)
        df = calculate_distance(df)
        df, avg_pace, fastest_pace, slowest_pace = calculate_pace(df, activity_type)
        pause_time = detect_pauses(df)
        avg_steps, total_steps = calculate_steps(df)

        # File storage paths
        track_img = os.path.join(IMG_DIR, f"{base_name}_track.png")
        elevation_img = os.path.join(IMG_DIR, f"{base_name}_elevation.svg")
        map_html = os.path.join(IMG_DIR, f"{base_name}_map.html")

        # Generate visualizations
        plot_track(df, track_img)
        plot_elevation(df, elevation_img)
        plot_activity_map(df, map_html)

        year, month = date.split("-")[:2]

        # Compute statistics and insert into DB
        run_data = compute_run_db_data(df, base_name, year, month, avg_steps, total_steps, avg_pace, fastest_pace,
                                       slowest_pace,
                                       pause_time, activity_type)
        insert_run(run_data, track_img, elevation_img, map_html)

        print(f"Successfully added {file} to the database.")

    print("Bulk upload completed!")


if __name__ == "__main__":
    bulk_upload_tcx()