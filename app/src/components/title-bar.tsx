import { SidebarTrigger } from "@/components/ui/sidebar";
import { isMac } from "@/lib/platform";
import { cn } from "@/lib/utils";

/**
 * Full-width title bar above the sidebar and content. On macOS the native title bar is an
 * overlay, so this bar is the drag region and leaves room on the left for the traffic lights.
 */
export function TitleBar({ title }: { title: string }) {
  return (
    <header
      data-tauri-drag-region
      className={cn(
        "fixed inset-x-0 top-0 z-20 flex h-(--titlebar-height) items-center gap-2 border-b bg-background pr-4 text-sm select-none",
        isMac ? "pl-20" : "pl-2",
      )}
    >
      <SidebarTrigger />
      <span data-tauri-drag-region className="font-medium">
        {title}
      </span>
    </header>
  );
}
