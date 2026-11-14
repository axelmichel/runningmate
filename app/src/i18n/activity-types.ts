import { tk } from "@/i18n";

/**
 * Activity type names that come from the database (`activities.activity_type` from TCX files
 * and the `type_detail` choices in `src-tauri/src/activity_type.rs`). Listed here so
 * `i18n.test.ts` checks that each has a German translation; they are shown with `t(name)`.
 */
export const ACTIVITY_TYPE_NAMES = [
  // Raw TCX sports grouped into the views
  tk("Running"),
  tk("Trailrun"),
  tk("Run"),
  tk("Trackrun"),
  tk("Track"),
  tk("Walking"),
  tk("Hike"),
  tk("Trekking"),
  tk("Other"),
  tk("Nordic Walking"),
  tk("Cycling"),
  tk("Bike"),
  tk("MTB"),
  tk("Bicycle"),
  tk("Biking"),
  tk("E-Bike"),
  tk("Gravelbike"),
  tk("Mountainbike"),
  // Type details chosen on the edit page
  tk("Trail"),
  tk("Endurance"),
  tk("Interval"),
  tk("Recovery"),
  tk("Speed"),
  tk("Long Run"),
  tk("Hiking"),
  tk("Mountain Biking"),
  tk("Gravel Biking"),
  tk("Default"),
];
