import pytest

from utils.save_avg import safe_avg


@pytest.mark.parametrize("data, expected", [
    ([1, 2, 3], 2.0),
    ([1, None, 3], 2.0),
    ([1, float("nan"), 3], 2.0),
    ([1, "2", "3.5"], 2.1666666666666665),
    ([5], 5.0),
    ([], None),
    ([0, "0", 0.0], 0.0),
    (["-1", "-2.5", 4], 0.16666666666666666),
])
def test_safe_avg(data, expected):
    result = safe_avg(data)
    if expected is None:
        assert result is None
    else:
        assert result == pytest.approx(expected)