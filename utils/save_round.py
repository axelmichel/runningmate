import pandas as pd


def safe_round(value, decimals=0):
    """Safely rounds a number, avoiding NaN and None issues."""
    if value is None or not isinstance(value, (int, float)) or pd.isna(value):
        return 0  # Return 0 if value is None or NaN
    return int(round(value, decimals))
