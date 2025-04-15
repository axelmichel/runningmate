#!/bin/bash

# Ensure locale directory exists
mkdir -p locales

# Find relevant .py files
# Find relevant .py files
find . -type f -name "*.py" \
  -not -path "*/venv/*" \
  -not -path "*/tests/*" \
  -not -path "*/dist/*" \
  -not -path "*/site/*" \
  -not -path "*/files/*" > to_translate.txt

# Extract translations
cat to_translate.txt | xargs pybabel extract -F babel.cfg -o locales/messages.pot

echo "Translations extracted to locales/messages.pot"

pybabel update -i locales/messages.pot -d locales