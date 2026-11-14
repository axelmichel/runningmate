import { useEffect, useState } from "react";

import { AppSidebar } from "@/components/app-sidebar";
import { TitleBar } from "@/components/title-bar";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import type { ViewMode } from "@/features/activities/view-mode";
import { api, type AppInfo } from "@/lib/api";

function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("All");
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    api.appInfo().then(setInfo).catch(console.error);
  }, []);

  return (
    <SidebarProvider>
      <AppSidebar viewMode={viewMode} onViewModeChange={setViewMode} />
      <TitleBar title={viewMode} />
      <SidebarInset className="pt-(--titlebar-height)">
        <main className="p-4">
          <Card>
            <CardHeader>
              <CardTitle>RunningMate</CardTitle>
              <CardDescription>Version {info?.version ?? "…"}</CardDescription>
            </CardHeader>
            <CardContent className="text-sm text-muted-foreground">
              Data folder: {info?.dataDir ?? "…"}
            </CardContent>
          </Card>
        </main>
      </SidebarInset>
    </SidebarProvider>
  );
}

export default App;
