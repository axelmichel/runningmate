from datetime import datetime


def calculate_age(birthday: str) -> int:
    """
    Calculates the age based on a given birthdate in the format DD.MM.YYYY.

    Params:
        birthday (str): The birthdate string in format 'DD.MM.YYYY'.

    Returns:
        int: The calculated age.
    """
    try:
        birth_date = datetime.strptime(birthday, "%d.%m.%Y")
        today = datetime.today()
        age = today.year - birth_date.year - ((today.month, today.day) < (birth_date.month, birth_date.day))
        return age
    except ValueError:
        return 0