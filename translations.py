import gettext
import locale
import os

# Get system default language
default_lang, _ = locale.getlocale()

# Define where translation files are stored
LOCALE_DIR = os.path.join(os.path.dirname(__file__), "locales")

# Function to load translations dynamically
def set_language(lang_code=None):
    """Set the application language dynamically."""
    lang_code = lang_code or default_lang  # Use provided language or system default
    try:
        lang = gettext.translation("messages", localedir=LOCALE_DIR, languages=[lang_code])
        lang.install()
        return lang.gettext  # Return the gettext function
    except FileNotFoundError:
        return lambda s: s  # Fallback: Return string as-is

# Initialize translations (Global `_` function)
_ = set_language()