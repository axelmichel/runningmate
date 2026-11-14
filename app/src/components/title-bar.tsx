import { PanelRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { SidebarTrigger } from "@/components/ui/sidebar";
import { isMac } from "@/lib/platform";
import { t } from "@/i18n";
import { cn } from "@/lib/utils";

type TitleBarProps = {
  title: string;
  detailsOpen: boolean;
  onDetailsToggle: () => void;
};

/**
 * Full-width title bar above the sidebar and content. On macOS the native title bar is an
 * overlay, so this bar is the drag region and leaves room on the left for the traffic lights.
 * The left button toggles the sidebar, the right one the details panel.
 */
export function TitleBar({ title, detailsOpen, onDetailsToggle }: TitleBarProps) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "fixed inset-x-0 top-0 z-20 flex h-(--titlebar-height) items-center gap-2 border-b bg-background pr-2 text-sm select-none",
        isMac ? "pl-20" : "pl-2",
      )}
    >
      <SidebarTrigger />
      <span data-tauri-drag-region className="flex-1 font-medium">
        {title}
      </span>
      <Button variant="ghost" size="icon-sm" aria-pressed={detailsOpen} onClick={onDetailsToggle}>
        <PanelRight />
        <span className="sr-only">{t("Toggle details panel")}</span>
      </Button>
    </header>
  );
}
