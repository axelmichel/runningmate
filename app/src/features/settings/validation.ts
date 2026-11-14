/** Checks for the settings forms. Each returns a message per invalid field (empty when valid). */

export type ProfileForm = {
  name: string;
  weight: string;
  height: string;
  birthday: string;
  hrMin: string;
  hrMax: string;
};

export type Errors<T> = Partial<Record<keyof T, string>>;

const number = (value: string) => (value.trim() === "" ? Number.NaN : Number(value));

/** "DD.MM.YYYY" for a real past date, as stored by the Python app. */
export function isBirthday(value: string, today = new Date()): boolean {
  const match = value.trim().match(/^(\d{2})\.(\d{2})\.(\d{4})$/);
  if (!match) return false;
  const [day, month, year] = [Number(match[1]), Number(match[2]), Number(match[3])];
  const date = new Date(year, month - 1, day);
  return (
    date.getFullYear() === year &&
    date.getMonth() === month - 1 &&
    date.getDate() === day &&
    date < today &&
    year > 1900
  );
}

export function validateProfile(form: ProfileForm): Errors<ProfileForm> {
  const errors: Errors<ProfileForm> = {};
  const weight = number(form.weight);
  const height = number(form.height);
  const hrMin = number(form.hrMin);
  const hrMax = number(form.hrMax);
  if (!form.name.trim()) errors.name = "Enter a name.";
  if (!(weight >= 20 && weight <= 300)) errors.weight = "Weight in kg, between 20 and 300.";
  if (!(Number.isInteger(height) && height >= 100 && height <= 250)) {
    errors.height = "Height in cm, between 100 and 250.";
  }
  if (!isBirthday(form.birthday)) errors.birthday = "Use DD.MM.YYYY.";
  if (!(Number.isInteger(hrMin) && hrMin >= 30 && hrMin <= 120)) {
    errors.hrMin = "Resting heart rate between 30 and 120 bpm.";
  }
  if (!(Number.isInteger(hrMax) && hrMax >= 100 && hrMax <= 230)) {
    errors.hrMax = "Maximum heart rate between 100 and 230 bpm.";
  } else if (!errors.hrMin && hrMin >= hrMax) {
    errors.hrMax = "Must be above the resting heart rate.";
  }
  return errors;
}

/** Zone upper bounds must be whole bpm values and strictly ascending. */
export function validateZones(zones: string[]): string | null {
  const values = zones.map(number);
  if (values.some((v) => !Number.isInteger(v) || v < 40 || v > 240)) {
    return "Each zone needs a whole number between 40 and 240 bpm.";
  }
  if (values.some((v, i) => i > 0 && v <= values[i - 1])) {
    return "Each zone must end above the previous one.";
  }
  return null;
}

/** Bytes → "1.2 GB" style text. */
export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 || value >= 100 ? 0 : 1;
  return `${new Intl.NumberFormat(undefined, { maximumFractionDigits: digits }).format(value)} ${units[unit]}`;
}
