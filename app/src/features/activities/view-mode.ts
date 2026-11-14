import { Bike, Footprints, LayoutGrid, PersonStanding, type LucideIcon } from "lucide-react";

/** Mirrors `ViewMode` in the Python app (processing/system_settings.py). */
export type ViewMode = "All" | "Running" | "Cycling" | "Walking";

export const VIEW_MODES: { mode: ViewMode; label: string; icon: LucideIcon }[] = [
  { mode: "All", label: "All", icon: LayoutGrid },
  { mode: "Running", label: "Runs", icon: Footprints },
  { mode: "Cycling", label: "Rides", icon: Bike },
  { mode: "Walking", label: "Walks", icon: PersonStanding },
];
