/** How files from disk are added to an activity: copied, linked, or ask each time. */
export type MediaMode = "ask" | "copy" | "link";

const KEY = "runningmate-media-mode";

export function mediaMode(): MediaMode {
  const stored = localStorage.getItem(KEY);
  return stored === "copy" || stored === "link" ? stored : "ask";
}

export function setMediaMode(mode: MediaMode) {
  localStorage.setItem(KEY, mode);
}

/** Extensions the media importer accepts (`MediaType::from_extension` in Rust). */
export const MEDIA_EXTENSIONS = ["png", "jpg", "jpeg", "heic", "mp4", "mov", "avi"];

export function isMediaFile(path: string): boolean {
  const extension = path.split(".").pop()?.toLowerCase() ?? "";
  return MEDIA_EXTENSIONS.includes(extension);
}
