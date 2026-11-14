export type Connector = "garmin" | "folder" | "files";

export const LAST_CONNECTOR_KEY = "runningmate-last-connector";

/** The connector used last, so Sync reopens where the user left off. */
export function lastConnector(): Connector {
  const stored = localStorage.getItem(LAST_CONNECTOR_KEY);
  return stored === "folder" || stored === "files" ? stored : "garmin";
}
