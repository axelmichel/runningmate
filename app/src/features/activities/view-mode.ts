import { Bike, Footprints, LayoutGrid, type LucideIcon } from "lucide-react";

import type { ViewMode } from "@/bindings/ViewMode";
import { PersonWalking } from "@/components/icons";
import { tk } from "@/i18n";

export type { ViewMode };

/** Sidebar views; `label` is a translation key. */
export const VIEW_MODES: { mode: ViewMode; label: string; icon: LucideIcon }[] = [
  { mode: "All", label: tk("All"), icon: LayoutGrid },
  { mode: "Running", label: tk("Runs"), icon: Footprints },
  { mode: "Cycling", label: tk("Rides"), icon: Bike },
  { mode: "Walking", label: tk("Walks"), icon: PersonWalking },
];
