import { FileUp, FolderSync, Watch } from "lucide-react";
import { useCallback, useState, type ReactNode } from "react";

import { NavTabs, type NavItem } from "@/components/nav-tabs";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { t, tk } from "@/i18n";

import { LAST_CONNECTOR_KEY, type Connector } from "./connector";
import { FileImportPanel } from "./file-import-panel";
import { FolderSyncPanel } from "./folder-sync-panel";
import { GarminPanel } from "./garmin-panel";

type ConnectorInfo = NavItem & {
  value: Connector;
  title: string;
  description: string;
  /** What the connector can do; translation keys. */
  capabilities: string[];
};

const CONNECTORS: ConnectorInfo[] = [
  {
    value: "garmin",
    label: tk("Garmin Connect"),
    title: "Garmin Connect",
    icon: Watch,
    description: tk(
      "Two-way: new Garmin activities are imported, activities from elsewhere can be uploaded.",
    ),
    capabilities: [tk("Import"), tk("Export")],
  },
  {
    value: "folder",
    label: tk("Folder sync"),
    title: tk("Folder sync"),
    icon: FolderSync,
    description: tk(
      "Imports new TCX files from a folder, e.g. an iCloud export of RunGap or HealthFit.",
    ),
    capabilities: [tk("Import")],
  },
  {
    value: "files",
    label: tk("TCX files"),
    title: tk("TCX files"),
    icon: FileUp,
    description: tk(
      "Imports TCX files you pick. They are archived in the imports folder; the originals stay where they are.",
    ),
    capabilities: [tk("Import")],
  },
];

type ConnectorsDialogProps = {
  /** The connector to show, or `null` when closed. */
  connector: Connector | null;
  onConnectorChange: (connector: Connector | null) => void;
};

/** Services and sources RunningMate syncs with, one page per connector. */
export function ConnectorsDialog({ connector, onConnectorChange }: ConnectorsDialogProps) {
  const [running, setRunning] = useState(false);
  const onRunningChange = useCallback((value: boolean) => setRunning(value), []);
  const current = CONNECTORS.find((c) => c.value === connector) ?? CONNECTORS[0];
  const panels: Record<Connector, ReactNode> = {
    garmin: <GarminPanel onRunningChange={onRunningChange} />,
    folder: <FolderSyncPanel onRunningChange={onRunningChange} />,
    files: <FileImportPanel onRunningChange={onRunningChange} />,
  };

  return (
    <Dialog
      open={connector !== null}
      onOpenChange={(open) => !open && !running && onConnectorChange(null)}
    >
      <DialogContent
        showCloseButton={!running}
        className="flex h-[75vh] max-w-4xl flex-col gap-4 sm:max-w-4xl"
      >
        <DialogHeader>
          <DialogTitle>{t("Connectors")}</DialogTitle>
          <DialogDescription>
            {t("Services and folders RunningMate imports from or exports to.")}
          </DialogDescription>
        </DialogHeader>
        <Tabs
          value={current.value}
          // Switching while a sync runs would abandon its progress view.
          onValueChange={(value) => {
            if (running) return;
            localStorage.setItem(LAST_CONNECTOR_KEY, value);
            onConnectorChange(value as Connector);
          }}
          orientation="vertical"
          className="min-h-0 flex-1 flex-row gap-6"
        >
          <div className="grid content-start gap-4">
            <NavTabs items={CONNECTORS} />
            <p className="w-48 text-xs text-muted-foreground">
              {t(
                "Strava and others need a registered API app of their own and can be added later.",
              )}
            </p>
          </div>
          <div className="scrollbar-visible min-h-0 min-w-0 flex-1 overflow-auto pr-1">
            {CONNECTORS.map((c) => (
              <TabsContent key={c.value} value={c.value} className="grid content-start gap-4">
                <div className="grid gap-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <h2 className="text-base font-semibold">{t(c.title)}</h2>
                    {c.capabilities.map((capability) => (
                      <Badge key={capability} variant="secondary">
                        {t(capability)}
                      </Badge>
                    ))}
                  </div>
                  <p className="text-sm text-muted-foreground">{t(c.description)}</p>
                </div>
                {panels[c.value]}
              </TabsContent>
            ))}
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
