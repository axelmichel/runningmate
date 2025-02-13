import folium
import matplotlib.pyplot as plt


def plot_track(df, output_path):
    """Creates a 2D view of the route without distortion and saves it."""
    fig, ax = plt.subplots(figsize=(10, 10))  # Keep a square figure
    ax.plot(df["X"], df["Y"], color="white", linewidth=2)

    # Ensure equal aspect ratio to prevent distortion
    ax.set_aspect("equal", adjustable="datalim")

    # Remove axis labels and ticks for a clean look
    ax.set_xticks([])
    ax.set_yticks([])
    ax.set_frame_on(False)

    # Transparent background
    fig.patch.set_alpha(0)
    ax.patch.set_alpha(0)

    # Save with tight bounding box and high resolution
    plt.savefig(
        output_path, dpi=300, transparent=True, bbox_inches="tight", pad_inches=0
    )
    plt.close(fig)


def plot_activity_map(df, output_path):
    """Plots the activity route on an OpenStreetMap and saves it as a PNG file."""

    # Get the starting coordinates for the map center
    start_lat, start_lon = df.iloc[0]["Latitude"], df.iloc[0]["Longitude"]

    # Create a folium map centered around the starting point
    activity_map = folium.Map(
        location=[start_lat, start_lon], zoom_start=14, tiles="cartodbpositron"
    )

    # Extract route coordinates
    route = list(zip(df["Latitude"], df["Longitude"]))

    # Add the route as a polyline
    folium.PolyLine(route, color="blue", weight=4, opacity=0.7).add_to(activity_map)

    # Save the map to an HTML file (temporary)
    html_path = output_path.replace(".png", ".html")
    activity_map.save(html_path)


def plot_elevation(df, output_path):
    """Creates an elevation profile of the route as an SVG with a transparent background and white elements."""
    fig, ax = plt.subplots(figsize=(10, 5))

    # Set figure background to transparent
    fig.patch.set_alpha(0)
    ax.patch.set_alpha(0)

    # Plot the elevation profile with white color
    ax.plot(df["Distance"], df["Elevation"], color="white", linewidth=2)

    # Set labels and text to white
    ax.set_xlabel("Distance (km)", color="white")
    ax.set_ylabel("Height (m)", color="white")

    # Remove unnecessary spines
    ax.spines["right"].set_visible(False)
    ax.spines["top"].set_visible(False)

    # Set remaining spines to white
    ax.spines["left"].set_color("white")
    ax.spines["bottom"].set_color("white")

    # Set tick labels to white
    ax.tick_params(axis="x", colors="white")
    ax.tick_params(axis="y", colors="white")

    # Save with transparent background
    plt.savefig(
        output_path, format="svg", bbox_inches="tight", pad_inches=0, transparent=True
    )
    plt.close(fig)
