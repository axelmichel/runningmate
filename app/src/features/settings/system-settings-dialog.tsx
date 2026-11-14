import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { HardDrive, SlidersHorizontal } from "lucide-react";
import { useState } from "react";

import { mediaMode, setMediaMode, type MediaMode } from "@/features/detail/media-mode";
import { toast } from "sonner";

import type { SystemSettings } from "@/bindings/SystemSettings";
import { NavTabs, type NavItem } from "@/components/nav-tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { useTheme, type Theme } from "@/hooks/use-theme";
import { setLanguage, t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";

import { formatBytes } from "./validation";

type Page = "general" | "storage";

const ITEMS: (NavItem & { value: Page })[] = [
  { value: "general", label: tk("General"), icon: SlidersHorizontal },
  { value: "storage", label: tk("Storage"), icon: HardDrive },
];

const LANGUAGES = [
  ["en", "English"],
  ["de", "Deutsch"],
  ["fr", "Français"],
] as const;

const THEMES: [Theme, string][] = [
  ["system", tk("Follow the system")],
  ["light", tk("Light")],
  ["dark", tk("Dark")],
];

const MEDIA_MODES: [MediaMode, string][] = [
  ["ask", tk("Ask each time")],
  ["copy", tk("Always copy into RunningMate")],
  ["link", tk("Always link to the originals")],
];

/** How photos and videos from disk are added (stored locally, not in settings.json). */
function MediaModeSetting() {
  const [mode, setMode] = useState<MediaMode>(mediaMode);
  return (
    <div className="grid gap-2">
      <Label>{t("Adding photos and videos from files")}</Label>
      <Select
        value={mode}
        onValueChange={(value) => {
          setMediaMode(value as MediaMode);
          setMode(value as MediaMode);
        }}
      >
        <SelectTrigger className="w-72">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {MEDIA_MODES.map(([value, label]) => (
            <SelectItem key={value} value={value}>
              {t(label)}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        {t("Photos from the Photos library are always copied.")}
      </p>
    </div>
  );
}

function GeneralPage() {
  const queryClient = useQueryClient();
  const { setTheme } = useTheme();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  // Changes save immediately, like the theme toggle in most apps.
  const save = useMutation({
    mutationFn: (next: SystemSettings) => api.saveSettings(next),
    onSuccess: (saved) => {
      queryClient.setQueryData(["settings"], saved);
      setTheme(saved.layout as Theme);
      setLanguage(saved.language);
    },
    onError: (error) =>
      toast.error(t("Could not save settings: {{error}}", { error: String(error) })),
  });
  if (!settings) return null;

  return (
    <div className="grid max-w-md gap-6">
      <div className="grid gap-2">
        <Label>{t("Language")}</Label>
        <Select
          value={settings.language}
          onValueChange={(language) => save.mutate({ ...settings, language })}
        >
          <SelectTrigger className="w-56">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {LANGUAGES.map(([code, label]) => (
              <SelectItem key={code} value={code}>
                {label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {t(
            "Also used for the titles of newly imported activities. French falls back to English.",
          )}
        </p>
      </div>
      <div className="grid gap-2">
        <Label>{t("Appearance")}</Label>
        <RadioGroup
          value={settings.layout}
          onValueChange={(layout) => save.mutate({ ...settings, layout })}
        >
          {THEMES.map(([value, label]) => (
            <div key={value} className="flex items-center gap-2">
              <RadioGroupItem value={value} id={`theme-${value}`} />
              <Label htmlFor={`theme-${value}`} className="font-normal">
                {t(label)}
              </Label>
            </div>
          ))}
        </RadioGroup>
      </div>
      <MediaModeSetting />
    </div>
  );
}

function StoragePage() {
  const { data: info } = useQuery({ queryKey: ["storage"], queryFn: api.storageInfo });
  if (!info) return null;
  const rows = [
    [t("Data folder"), info.dataDir],
    [
      t("Photos and videos"),
      t("{{count}} files · {{size}}", {
        count: info.mediaFiles,
        files: formatNumber(info.mediaFiles),
        size: formatBytes(info.mediaSize),
      }),
    ],
    [t("Total size"), formatBytes(info.totalSize)],
  ];
  return (
    <dl className="grid max-w-xl gap-3 text-sm">
      {rows.map(([label, value]) => (
        <div key={label} className="grid gap-0.5">
          <dt className="text-muted-foreground">{label}</dt>
          <dd className="font-medium break-all">{value}</dd>
        </div>
      ))}
      <p className="text-xs text-muted-foreground">
        {t(
          "All data stays on this computer. The folder is shared with the previous Python version of RunningMate.",
        )}
      </p>
    </dl>
  );
}

/** Language, appearance and storage (`ui/window_system_settings.py`). */
export function SystemSettingsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [page, setPage] = useState<Page>("general");
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex h-[60vh] max-w-3xl flex-col gap-4 sm:max-w-3xl">
        <DialogHeader>
          <DialogTitle>{t("Settings")}</DialogTitle>
          <DialogDescription className="sr-only">
            {t("Language, appearance and storage")}
          </DialogDescription>
        </DialogHeader>
        <Tabs
          value={page}
          onValueChange={(value) => setPage(value as Page)}
          orientation="vertical"
          className="min-h-0 flex-1 flex-row gap-6"
        >
          <NavTabs items={ITEMS} />
          <div className="min-h-0 min-w-0 flex-1 overflow-auto">
            <TabsContent value="general">
              <GeneralPage />
            </TabsContent>
            <TabsContent value="storage">
              <StoragePage />
            </TabsContent>
          </div>
        </Tabs>
      </DialogContent>
    </Dialog>
  );
}
