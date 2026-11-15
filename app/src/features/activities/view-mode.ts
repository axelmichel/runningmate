import { Bike, LayoutGrid, type LucideIcon } from "lucide-react";

import type { ViewMode } from "@/bindings/ViewMode";
import { PersonRunning, PersonWalking } from "@/components/icons";
import { tk } from "@/i18n";

export type { ViewMode };

/** Sidebar views; `label` is a translation key. */
export const VIEW_MODES: { mode: ViewMode; label: string; icon: LucideIcon }[] = [
  { mode: "All", label: tk("All"), icon: LayoutGrid },
  { mode: "Running", label: tk("Runs"), icon: PersonRunning },
  { mode: "Cycling", label: tk("Rides"), icon: Bike },
  { mode: "Walking", label: tk("Walks"), icon: PersonWalking },
];

/** One colour per sport, as in the charts; `All` uses the primary colour. */
export const SPORT_COLORS: Record<ViewMode, string> = {
  All: "var(--primary)",
  Running: "var(--sport-running)",
  Cycling: "var(--sport-cycling)",
  Walking: "var(--sport-walking)",
};
