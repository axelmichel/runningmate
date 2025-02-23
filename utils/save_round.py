import math


def safe_round(value, decimals=0):
    """Safely rounds a number, avoiding NaN and None issues."""
    if value is None or math.isnan(value) or math.isinf(value):
        return 0.0 if decimals > 0 else 0
    return round(value, decimals) if decimals > 0 else int(round(value))
