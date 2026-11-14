import { useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";

import { AppSidebar } from "@/components/app-sidebar";
import { TitleBar } from "@/components/title-bar";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { ActivitiesPage } from "@/features/activities/activities-page";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { api } from "@/lib/api";

const DETAILS_STORAGE_KEY = "runningmate-details-panel";

function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("All");
  const [searchOpen, setSearchOpen] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(
    () => localStorage.getItem(DETAILS_STORAGE_KEY) !== "closed",
  );

  const toggleDetails = () =>
    setDetailsOpen((open) => {
      localStorage.setItem(DETAILS_STORAGE_KEY, open ? "closed" : "open");
      return !open;
    });
  const { data: info } = useQuery({ queryKey: ["appInfo"], queryFn: api.appInfo });

  // Cmd/Ctrl+F toggles the search bar, as View → Search did.
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "f") {
        event.preventDefault();
        setSearchOpen((open) => !open);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  const title = VIEW_MODES.find((v) => v.mode === viewMode)?.label ?? viewMode;

  return (
    <SidebarProvider>
      <TitleBar title={title} detailsOpen={detailsOpen} onDetailsToggle={toggleDetails} />
      <AppSidebar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
        searchOpen={searchOpen}
        onSearchToggle={() => setSearchOpen((open) => !open)}
      />
      <SidebarInset className="h-svh overflow-hidden pt-(--titlebar-height)">
        <ActivitiesPage
          view={viewMode}
          searchOpen={searchOpen}
          onSearchOpenChange={setSearchOpen}
          detailsOpen={detailsOpen}
          dataDir={info?.dataDir}
        />
      </SidebarInset>
    </SidebarProvider>
  );
}

export default App;
