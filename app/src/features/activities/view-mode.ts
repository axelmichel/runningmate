import { Bike, Footprints, LayoutGrid, PersonStanding, type LucideIcon } from "lucide-react";

import type { ViewMode } from "@/bindings/ViewMode";

export type { ViewMode };

export const VIEW_MODES: { mode: ViewMode; label: string; icon: LucideIcon }[] = [
  { mode: "All", label: "All", icon: LayoutGrid },
  { mode: "Running", label: "Runs", icon: Footprints },
  { mode: "Cycling", label: "Rides", icon: Bike },
  { mode: "Walking", label: "Walks", icon: PersonStanding },
];
