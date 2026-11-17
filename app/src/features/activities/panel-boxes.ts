import type { PanelBox } from "@/bindings/PanelBox";
import type { PanelHidden } from "@/bindings/PanelHidden";
import type { SystemSettings } from "@/bindings/SystemSettings";
import type { ViewMode } from "@/bindings/ViewMode";
import { tk } from "@/i18n";

export type { PanelBox };

/** Boxes of the activity panel in their order; `label` is a translation key. */
export const PANEL_BOXES: { box: PanelBox; label: string }[] = [
  { box: "map", label: tk("Route map") },
  { box: "links", label: tk("Garmin Connect link") },
  { box: "achievements", label: tk("Goals, sessions and trophies") },
  { box: "notes", label: tk("Photos, comment and summary") },
  { box: "weather", label: tk("Weather") },
  { box: "best", label: tk("Best segments") },
];

/** Sports whose boxes can be picked; other activities show every box. */
export const PANEL_SPORTS = ["Running", "Cycling", "Walking"] as const;
export type PanelSport = (typeof PANEL_SPORTS)[number];

const KEYS: Record<PanelSport, keyof PanelHidden> = {
  Running: "running",
  Cycling: "cycling",
  Walking: "walking",
};

/** Boxes left out for activities of `view`. */
export function hiddenBoxes(settings: SystemSettings | undefined, view: ViewMode): PanelBox[] {
  if (!settings || view === "All") return [];
  return settings.activity_panel_hidden[KEYS[view]];
}

/** `hidden` with `box` shown or left out for `sport`. */
export function withBox(
  hidden: PanelHidden,
  sport: PanelSport,
  box: PanelBox,
  shown: boolean,
): PanelHidden {
  const key = KEYS[sport];
  const rest = hidden[key].filter((b) => b !== box);
  return { ...hidden, [key]: shown ? rest : [...rest, box] };
}
