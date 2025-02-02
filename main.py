import pandas as pd
import matplotlib.pyplot as plt
import xml.etree.ElementTree as ET
from pyproj import Transformer
import os
import re
import numpy as np
from dotenv import load_dotenv

load_dotenv('.env.local')

def parse_tcx(file_path):
    """Reads a TCX file and extracts GPS coordinates, elevation, heart rate, and other metrics."""
    tree = ET.parse(file_path)
    root = tree.getroot()
    namespaces = {'tcx': 'http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2'}

    trackpoints = []
    heart_rates = []
    for trackpoint in root.findall(".//tcx:Trackpoint", namespaces):
        lat = trackpoint.find(".//tcx:LatitudeDegrees", namespaces)
        lon = trackpoint.find(".//tcx:LongitudeDegrees", namespaces)
        ele = trackpoint.find(".//tcx:AltitudeMeters", namespaces)
        time = trackpoint.find(".//tcx:Time", namespaces)
        heart_rate = trackpoint.find(".//tcx:HeartRateBpm/tcx:Value", namespaces)
        extensions = trackpoint.find(".//tcx:Extensions", namespaces)
        if heart_rate is not None:
            heart_rates.append(int(heart_rate.text))
        steps = None
        power = None
        if extensions is not None:
            tpx = extensions.find(".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}TPX")
            if tpx is not None:
                steps = tpx.find(".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}RunCadence")
                power = tpx.find(
                    ".//{http://www.garmin.com/xmlschemas/ActivityExtension/v2}Watts")  # Schritte pro Minute

        if lat is not None and lon is not None and ele is not None and time is not None:
            trackpoints.append((time.text, float(lat.text), float(lon.text), float(ele.text),
                                int(steps.text) if steps is not None else None,
                                int(power.text) if power is not None else None))

    df = pd.DataFrame(trackpoints, columns=["Time", "Latitude", "Longitude", "Elevation", "Steps", "Power"])
    df["HeartRate"] = pd.Series(heart_rates) if heart_rates else pd.Series(dtype='float')
    return df


def convert_to_utm(df):
    """Converts GPS coordinates (Lat, Lon) into UTM coordinates for distortion removal"""
    lat = df["Latitude"].mean()
    lon = df["Longitude"].mean()
    utm_zone = int((lon + 180) / 6) + 1
    proj_string = f"+proj=utm +zone={utm_zone} +datum=WGS84 +units=m +no_defs"

    transformer = Transformer.from_crs("EPSG:4326", proj_string, always_xy=True)
    df["X"], df["Y"] = zip(*df.apply(lambda row: transformer.transform(row["Longitude"], row["Latitude"]), axis=1))

    return df


def calculate_distance(df):
    """Calculates the cumulative distance in kilometers along the route."""
    df["Distance"] = np.sqrt(np.diff(df["X"], prepend=df["X"].iloc[0])**2 +
                              np.diff(df["Y"], prepend=df["Y"].iloc[0])**2).cumsum() / 1000  # Umwandlung in km
    return df


def plot_track(df, output_path):
    """Creates a 2D view of the route without distortion and saves it."""
    fig, ax = plt.subplots(figsize=(10, 10))
    ax.plot(df["X"], df["Y"], color="white", linewidth=2)

    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_frame_on(False)

    fig.patch.set_alpha(0)
    ax.patch.set_alpha(0)

    plt.savefig(output_path, dpi=300, transparent=True, bbox_inches='tight', pad_inches=0)
    plt.close(fig)


def plot_elevation(df, output_path):
    """Creates an elevation profile of the route as an SVG and saves it."""
    fig, ax = plt.subplots(figsize=(10, 5))
    ax.plot(df["Distance"], df["Elevation"], color="black", linewidth=2)

    ax.set_xlabel("Distanz (km)")
    ax.set_ylabel("Höhe (m)")
    ax.set_title("Höhenprofil")
    ax.spines['right'].set_visible(False)
    ax.spines['top'].set_visible(False)

    plt.savefig(output_path, format="svg", bbox_inches='tight', pad_inches=0)
    plt.close(fig)


def extract_base_filename(file_name):
    """Extract date and time from filename."""
    match = re.match(r"(\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})", file_name)
    return match.group(1) if match else None


def write_to_excel(data, file_name):
    """Writes the running data into an Excel file per year and a sheet per month."""
    base_dir = os.path.dirname(os.path.abspath(__file__))
    year, month = file_name.split("-")[:2]
    excel_dir = os.path.join(base_dir, os.getenv("EXCEL_DIR"))
    if not os.path.exists(excel_dir):
        os.makedirs(excel_dir)
    excel_path = os.path.join(excel_dir, f"{year}.xlsx")
    sheet_name = f"{month}"

    # Bestehende Datei laden oder neue erstellen
    if os.path.exists(excel_path):
        with pd.ExcelWriter(excel_path, mode='a', if_sheet_exists='overlay') as writer:
            df_existing = pd.read_excel(writer,
                                        sheet_name=sheet_name) if sheet_name in writer.sheets else pd.DataFrame()
            df = pd.DataFrame([data])
            df = pd.concat([df_existing, df], ignore_index=True).drop_duplicates()
            df = df.sort_values(by=["Datum", "Startzeit (HH:mm)"], ascending=[True, True])
            df.to_excel(writer, sheet_name=sheet_name, index=False)
    else:
        df = pd.DataFrame([data])
        with pd.ExcelWriter(excel_path) as writer:
            df.to_excel(writer, sheet_name=sheet_name, index=False)


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

    for file_name in os.listdir(tcx_dir):
        if file_name.endswith(".tcx"):
            base_name = extract_base_filename(file_name)
            if base_name:
                output_track = os.path.join(png_dir, base_name + "_track.png")
                output_elevation = os.path.join(svg_dir, base_name + "_elevation.svg")

                if not os.path.exists(output_track) or not os.path.exists(output_elevation):
                    file_path = os.path.join(tcx_dir, file_name)
                    print(f"Processing file: {file_name}")
                    df = parse_tcx(file_path)
                    df = convert_to_utm(df)
                    df = calculate_distance(df)
                    plot_track(df, output_track)
                    plot_elevation(df, output_elevation)
                    print(f"images generated and saved: {output_track}, {output_elevation}")

                    # Berechnung der Laufdaten

                    total_distance = df["Distance"].iloc[-1]
                    total_time = pd.to_datetime(df["Time"].iloc[-1]) - pd.to_datetime(df["Time"].iloc[0])
                    avg_speed = total_distance / (total_time.total_seconds() / 3600)  # km/h
                    avg_steps = df["Steps"].mean()
                    avg_power = df["Power"].mean()
                    total_steps = df["Steps"].sum()

                    data = {
                        "Datum": base_name.split("_")[0].replace("-", "."),
                        "Startzeit (HH:mm)": pd.to_datetime(df["Time"].iloc[0]).strftime("%H:%M"),
                        "Distanz (km)": round(total_distance, 2),
                        "Zeit (HH:mm:ss)": str(total_time).split()[2] if total_time else "00:00:00",
                        "Höhenmeter": int(round(df["Elevation"].diff().clip(lower=0).sum(), 0)),
                        "Durchschnittsgeschwindigkeit (km/h)": round(avg_speed, 2),
                        "Durchschnitt Schritte (SPM)": int(round(avg_steps, 0)) if not np.isnan(
                            avg_steps) else 0 if not np.isnan(avg_steps) else 0,
                        "Durchschnitt Power (Watt)": int(round(avg_power, 0)) if not np.isnan(avg_power) else 0,
                        "Durchschnitt Herzfrequenz (BPM)": int(round(df["HeartRate"].mean(), 0)) if not df[
                            "HeartRate"].isnull().all() else 0 if not df["HeartRate"].isnull().all() else 0,
                        "Gesamtschritte": int(total_steps) if not np.isnan(total_steps) else 0
                    }
                    write_to_excel(data, base_name)
                    print(f"Data for {file_name} saved into an excel worksheet.")

                else:
                    print(f"{file_name} already processed.")


# Skript starten
if __name__ == "__main__":
    process_all_tcx_files()
