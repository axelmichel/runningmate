import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { AppSidebar } from "@/components/app-sidebar";
import { TitleBar } from "@/components/title-bar";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { AboutDialog } from "@/features/about-dialog";
import { ActivitiesPage } from "@/features/activities/activities-page";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { lastConnector, type Connector } from "@/features/connectors/connector";
import { ConnectorsDialog } from "@/features/connectors/connectors-dialog";
import { SystemSettingsDialog } from "@/features/settings/system-settings-dialog";
import { UserSettingsDialog } from "@/features/settings/user-settings-dialog";
import { useTheme, type Theme } from "@/hooks/use-theme";
import { setLanguage, t } from "@/i18n";
import { api } from "@/lib/api";

const DETAILS_STORAGE_KEY = "runningmate-details-panel";

function App() {
  const queryClient = useQueryClient();
  const [viewMode, setViewMode] = useState<ViewMode>("All");
  const [searchOpen, setSearchOpen] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(
    () => localStorage.getItem(DETAILS_STORAGE_KEY) !== "closed",
  );
  const [connector, setConnector] = useState<Connector | null>(null);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [userSettingsOpen, setUserSettingsOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { setTheme } = useTheme();
  // Re-renders the whole app when the language changes (see src/i18n).
  useTranslation();
  const { data: info } = useQuery({ queryKey: ["appInfo"], queryFn: api.appInfo });

  const toggleDetails = () =>
    setDetailsOpen((open) => {
      localStorage.setItem(DETAILS_STORAGE_KEY, open ? "closed" : "open");
      return !open;
    });

  // settings.json is the source of truth for the theme (shared with the Python app).
  useEffect(() => {
    api
      .getSettings()
      .then((settings) => {
        setTheme(settings.layout as Theme);
        setLanguage(settings.language);
      })
      .catch(() => {});
  }, [setTheme]);

  // Native menu items (src-tauri/src/menu.rs) arrive as `menu` events.
  useEffect(() => {
    const unlisten = listen<string>("menu", ({ payload }) => {
      if (payload === "sync") setConnector(lastConnector());
      if (payload === "about") setAboutOpen(true);
      if (payload === "search") setSearchOpen((open) => !open);
      if (payload === "settings") setSettingsOpen(true);
      if (payload === "user") setUserSettingsOpen(true);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  // Weather is fetched in the background after an import.
  useEffect(() => {
    const unlisten = listen<number>("activity://updated", ({ payload }) =>
      queryClient.invalidateQueries({ queryKey: ["weather", payload] }),
    );
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [queryClient]);

  const title = t(VIEW_MODES.find((v) => v.mode === viewMode)?.label ?? viewMode);

  return (
    <SidebarProvider>
      <TitleBar title={title} detailsOpen={detailsOpen} onDetailsToggle={toggleDetails} />
      <AppSidebar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
        searchOpen={searchOpen}
        onSearchToggle={() => setSearchOpen((open) => !open)}
        onSync={() => setConnector(lastConnector())}
        onUserSettings={() => setUserSettingsOpen(true)}
        onSettings={() => setSettingsOpen(true)}
      />
      <SidebarInset className="h-svh min-w-0 overflow-hidden pt-(--titlebar-height)">
        <ActivitiesPage
          view={viewMode}
          searchOpen={searchOpen}
          onSearchOpenChange={setSearchOpen}
          detailsOpen={detailsOpen}
          dataDir={info?.dataDir}
        />
      </SidebarInset>
      <ConnectorsDialog connector={connector} onConnectorChange={setConnector} />
      <AboutDialog open={aboutOpen} onOpenChange={setAboutOpen} />
      <UserSettingsDialog open={userSettingsOpen} onOpenChange={setUserSettingsOpen} />
      <SystemSettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </SidebarProvider>
  );
}

export default App;
