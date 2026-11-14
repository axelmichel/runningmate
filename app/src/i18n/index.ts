/**
 * Translations. Keys are the English texts, so a missing German string falls back to readable
 * English. `de.json` holds German; `en.json` only holds English plural forms.
 *
 * Components call the exported `t`. `App` subscribes to language changes with
 * `useTranslation()`, and since nothing below it is memoised, the whole tree re-renders in the
 * new language. Texts in module-level tables are marked with `tk()` and translated with `t()`
 * where they are rendered; `i18n.test.ts` checks that every marked text has a German entry.
 */
import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import { setFormatLocale } from "@/lib/format";

import de from "./de.json";
import en from "./en.json";

export type Language = "en" | "de" | "fr";

const STORAGE_KEY = "runningmate-language";
// Not available in unit tests.
const storage = typeof localStorage === "undefined" ? undefined : localStorage;

i18n.use(initReactI18next).init({
  resources: { en: { translation: en }, de: { translation: de } },
  lng: storage?.getItem(STORAGE_KEY) ?? "en",
  // French has no translations yet and falls back to English, as in the Python app.
  fallbackLng: "en",
  keySeparator: false,
  nsSeparator: false,
  interpolation: { escapeValue: false },
  returnEmptyString: false,
});
setFormatLocale(i18n.language);

export const t = i18n.t.bind(i18n);

/** Marks a text in a module-level table for translation; translate it with `t()` when shown. */
export const tk = <T extends string>(key: T): T => key;

export function setLanguage(language: string) {
  storage?.setItem(STORAGE_KEY, language);
  setFormatLocale(language);
  void i18n.changeLanguage(language);
}

export default i18n;
