import { useQuery, useQueryClient } from "@tanstack/react-query";
import { listen } from "@tauri-apps/api/event";
import { open as pickFiles } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useState } from "react";

import { AppSidebar } from "@/components/app-sidebar";
import { TitleBar } from "@/components/title-bar";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { ActivitiesPage } from "@/features/activities/activities-page";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { FolderSyncDialog } from "@/features/import/folder-sync-dialog";
import { ImportDialog } from "@/features/import/import-dialog";
import { useImportRun } from "@/features/import/use-import-run";
import { SystemSettingsDialog } from "@/features/settings/system-settings-dialog";
import { UserSettingsDialog } from "@/features/settings/user-settings-dialog";
import { useTheme, type Theme } from "@/hooks/use-theme";
import { api } from "@/lib/api";

const DETAILS_STORAGE_KEY = "runningmate-details-panel";

function App() {
  const queryClient = useQueryClient();
  const [viewMode, setViewMode] = useState<ViewMode>("All");
  const [searchOpen, setSearchOpen] = useState(false);
  const [detailsOpen, setDetailsOpen] = useState(
    () => localStorage.getItem(DETAILS_STORAGE_KEY) !== "closed",
  );
  const [importOpen, setImportOpen] = useState(false);
  const [folderSyncOpen, setFolderSyncOpen] = useState(false);
  const [userSettingsOpen, setUserSettingsOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { setTheme } = useTheme();
  const importRun = useImportRun();
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
      .then((settings) => setTheme(settings.layout as Theme))
      .catch(() => {});
  }, [setTheme]);

  const { start: startImport } = importRun;
  const importTcx = useCallback(async () => {
    const selected = await pickFiles({
      multiple: true,
      filters: [{ name: "TCX files", extensions: ["tcx"] }],
    });
    if (!selected || selected.length === 0) return;
    setImportOpen(true);
    await startImport(() => api.importFiles(selected));
  }, [startImport]);

  // Native menu items (src-tauri/src/menu.rs) arrive as `menu` events.
  useEffect(() => {
    const unlisten = listen<string>("menu", ({ payload }) => {
      if (payload === "import") importTcx();
      if (payload === "folder-sync") setFolderSyncOpen(true);
      if (payload === "search") setSearchOpen((open) => !open);
      if (payload === "settings") setSettingsOpen(true);
      if (payload === "user") setUserSettingsOpen(true);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [importTcx]);

  // Weather is fetched in the background after an import.
  useEffect(() => {
    const unlisten = listen<number>("activity://updated", ({ payload }) =>
      queryClient.invalidateQueries({ queryKey: ["weather", payload] }),
    );
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [queryClient]);

  const title = VIEW_MODES.find((v) => v.mode === viewMode)?.label ?? viewMode;

  return (
    <SidebarProvider>
      <TitleBar title={title} detailsOpen={detailsOpen} onDetailsToggle={toggleDetails} />
      <AppSidebar
        viewMode={viewMode}
        onViewModeChange={setViewMode}
        searchOpen={searchOpen}
        onSearchToggle={() => setSearchOpen((open) => !open)}
        onImport={importTcx}
        onFolderSync={() => setFolderSyncOpen(true)}
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
      <ImportDialog
        open={importOpen}
        onOpenChange={(open) => {
          setImportOpen(open);
          if (!open) importRun.reset();
        }}
        run={importRun.run}
      />
      <FolderSyncDialog open={folderSyncOpen} onOpenChange={setFolderSyncOpen} />
      <UserSettingsDialog open={userSettingsOpen} onOpenChange={setUserSettingsOpen} />
      <SystemSettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />
    </SidebarProvider>
  );
}

export default App;
