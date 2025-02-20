import pandas as pd


def safe_avg(data_list):
    """
    Computes the average of a list while handling None, NaN, and non-numeric values.
    If a value is a string representing a number (e.g., "1.3"), it attempts conversion.
    """
    clean_list = []
    for x in data_list:
        if x is None or pd.isna(x):
            continue  # Skip None and NaN
        if isinstance(x, (int, float)):
            clean_list.append(x)
        elif isinstance(x, str):
            try:
                clean_list.append(float(x))  # Convert numeric strings
            except ValueError:
                continue  # Skip non-numeric strings

    return sum(clean_list) / len(clean_list) if clean_list else None
