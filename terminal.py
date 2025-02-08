import os
from dotenv import load_dotenv

from processing.compute_statistics import compute_run_statistics
from processing.parse_tcx import parse_tcx
from processing.data_processing import convert_to_utm, calculate_distance, calculate_pace, detect_pauses, \
    calculate_steps
from processing.visualization import plot_track, plot_elevation
from processing.excel_handler import write_to_excel
import re

load_dotenv('.env.local')

def delete_existing_excel_files():
    """Deletes all existing Excel records."""
    base_dir = os.path.dirname(os.path.abspath(__file__))
    excel_dir = os.path.join(base_dir, os.getenv("EXCEL_DIR"))

    if os.path.exists(excel_dir):
        for file in os.listdir(excel_dir):
            if file.endswith(".xlsx"):
                file_path = os.path.join(excel_dir, file)
                os.remove(file_path)
                print(f"{file} has been deleted.")
        print("All records have been deleted.")

def extract_base_filename(file_name):
    """Extract date and time from filename."""
    match = re.match(r"(\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})", file_name)
    return match.group(1) if match else None

def process_all_tcx_files():
    """Processes all TCX files in tcx-files for which no PNG exists yet."""
    base_dir = os.path.dirname(os.path.abspath(__file__))

    tcx_dir = os.path.join(base_dir, os.getenv("TCX_DIR"))
    png_dir = os.path.join(base_dir, os.getenv("PNG_DIR"))
    svg_dir = os.path.join(base_dir, os.getenv("SVG_DIR"))

    if not os.path.exists(png_dir):
        os.makedirs(png_dir)

    if not os.path.exists(svg_dir):
        os.makedirs(svg_dir)

    user_input = input("Do you want to reprocess all files? (yes/y/no/n): ").strip().lower()
    reprocess_all = user_input in ["yes", "y"]

    if reprocess_all:
        delete_existing_excel_files()

    for file_name in os.listdir(tcx_dir):
        if file_name.endswith(".tcx"):
            base_name = extract_base_filename(file_name)
            if base_name:
                output_track = os.path.join(png_dir, base_name + "_track.png")
                output_elevation = os.path.join(svg_dir, base_name + "_elevation.svg")
                output_path_excel = os.path.join(base_dir, os.getenv("EXCEL_DIR"))

                if reprocess_all or not os.path.exists(output_track) and os.path.exists(output_elevation):
                    file_path = os.path.join(tcx_dir, file_name)
                    print(f"Processing file: {file_name}")
                    df, activity_type = parse_tcx(file_path)

                    df = convert_to_utm(df)
                    df = calculate_distance(df)
                    df, avg_pace, fastest_pace, slowest_pace = calculate_pace(df, activity_type)

                    total_pause_time = detect_pauses(df)
                    avg_steps, total_steps = calculate_steps(df)


                    plot_track(df, output_track)
                    plot_elevation(df, output_elevation)
                    print(f"images generated and saved: {output_track}, {output_elevation}")

                    data = compute_run_statistics(df, base_name, avg_steps, total_steps, avg_pace, fastest_pace,
                                                  slowest_pace, total_pause_time, activity_type)

                    write_to_excel(data, base_name, output_path_excel)
                    print(f"Data for {file_name} saved into an excel worksheet.")

                else:
                    print(f"{file_name} already processed.")


# Start script
if __name__ == "__main__":
    process_all_tcx_files()
