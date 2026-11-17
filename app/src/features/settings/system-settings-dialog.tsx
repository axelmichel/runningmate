import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { HardDrive, Loader2, PanelRight, SlidersHorizontal, Sparkles } from "lucide-react";
import { useState } from "react";

import { mediaMode, setMediaMode, type MediaMode } from "@/features/detail/media-mode";
import { toast } from "sonner";

import type { SystemSettings } from "@/bindings/SystemSettings";
import { NavTabs, type NavItem } from "@/components/nav-tabs";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
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
import { PANEL_BOXES, PANEL_SPORTS, hiddenBoxes, withBox } from "@/features/activities/panel-boxes";
import { VIEW_MODES } from "@/features/activities/view-mode";
import { useTheme, type Theme } from "@/hooks/use-theme";
import { setLanguage, t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";

import { formatBytes } from "./validation";

type Page = "general" | "panel" | "ai" | "storage";

const ITEMS: (NavItem & { value: Page })[] = [
  { value: "general", label: tk("General"), icon: SlidersHorizontal },
  { value: "panel", label: tk("Activity panel"), icon: PanelRight },
  { value: "ai", label: tk("AI summary"), icon: Sparkles },
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

const PERIOD_WEEKS = [2, 4, 8, 12];

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

/**
 * The settings plus a mutation that saves them; changes save immediately. Each change is applied
 * to the latest saved settings and saves run one after another, so two quick changes (ticking a
 * box, then picking a model) can't overwrite each other with an outdated copy.
 */
function useSettings() {
  const queryClient = useQueryClient();
  const { setTheme } = useTheme();
  const { data: settings } = useQuery({ queryKey: ["settings"], queryFn: api.getSettings });
  const save = useMutation({
    scope: { id: "settings" },
    mutationFn: (
      change: Partial<SystemSettings> | ((current: SystemSettings) => Partial<SystemSettings>),
    ) => {
      const current = queryClient.getQueryData<SystemSettings>(["settings"]);
      if (!current) throw new Error("settings not loaded");
      return api.saveSettings({
        ...current,
        ...(typeof change === "function" ? change(current) : change),
      });
    },
    onSuccess: (saved) => {
      queryClient.setQueryData(["settings"], saved);
      setTheme(saved.layout as Theme);
      setLanguage(saved.language);
    },
    onError: (error) =>
      toast.error(t("Could not save settings: {{error}}", { error: String(error) })),
  });
  return { settings, save };
}

function GeneralPage() {
  const { settings, save } = useSettings();
  if (!settings) return null;

  return (
    <div className="grid max-w-md gap-6">
      <div className="grid gap-2">
        <Label>{t("Language")}</Label>
        <Select value={settings.language} onValueChange={(language) => save.mutate({ language })}>
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
        <RadioGroup value={settings.layout} onValueChange={(layout) => save.mutate({ layout })}>
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
      <div className="grid gap-2">
        <div className="flex items-center gap-2">
          <Checkbox
            id="location-lookup"
            checked={settings.location_lookup}
            onCheckedChange={(checked) => save.mutate({ location_lookup: checked === true })}
          />
          <Label htmlFor="location-lookup">{t("Look up place names")}</Label>
        </div>
        <p className="text-xs text-muted-foreground">
          {t(
            "Names the town and part of town an activity starts in, for the list and the search. Sends the start's coordinates to nominatim.openstreetmap.org, one activity a second, each only once.",
          )}
        </p>
      </div>
    </div>
  );
}

/** Which boxes the panel beside the activity list and in the detail window shows, per sport. */
function PanelPage() {
  const { settings, save } = useSettings();
  if (!settings) return null;
  const sports = PANEL_SPORTS.map((sport) => VIEW_MODES.find((v) => v.mode === sport)!);

  return (
    <div className="grid max-w-xl gap-4">
      <p className="text-sm text-muted-foreground">
        {t("Boxes shown beside the activity list and in the detail window.")}
      </p>
      <table className="text-sm">
        <thead>
          <tr className="text-muted-foreground">
            <th className="py-2 text-left font-normal" />
            {sports.map(({ mode, label, icon: Icon }) => (
              <th key={mode} className="w-20 py-2 font-normal">
                <span className="inline-flex flex-col items-center gap-1">
                  <Icon className="size-4" />
                  {t(label)}
                </span>
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y">
          {PANEL_BOXES.map(({ box, label }) => (
            <tr key={box}>
              <td className="py-2">{t(label)}</td>
              {PANEL_SPORTS.map((sport) => (
                <td key={sport} className="py-2 text-center">
                  <Checkbox
                    aria-label={`${t(label)} · ${t(sports.find((s) => s.mode === sport)!.label)}`}
                    checked={!hiddenBoxes(settings, sport).includes(box)}
                    onCheckedChange={(checked) =>
                      save.mutate((current) => ({
                        activity_panel_hidden: withBox(
                          current.activity_panel_hidden,
                          sport,
                          box,
                          checked === true,
                        ),
                      }))
                    }
                  />
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      <p className="text-xs text-muted-foreground">
        {t("Photos, comment and summary only show beside the activity list.")}
      </p>
    </div>
  );
}

/** Activity summaries from a local Ollama server. */
function AiPage() {
  const { settings, save } = useSettings();
  const [url, setUrl] = useState<string | null>(null);
  const models = useMutation({
    mutationFn: (server: string) => api.aiModels(server),
    onError: (error) =>
      toast.error(t("Could not reach Ollama: {{error}}", { error: String(error) })),
  });
  if (!settings) return null;
  const serverUrl = url ?? settings.ai_url;
  const saveUrl = () => {
    const trimmed = serverUrl.trim().replace(/\/+$/, "");
    if (trimmed !== settings.ai_url) save.mutate({ ai_url: trimmed });
    setUrl(null);
  };
  // The chosen model stays selectable even before the list was loaded.
  const choices = [...new Set([...(models.data ?? []), settings.ai_model].filter(Boolean))];

  return (
    <div className="grid max-w-md gap-6">
      <div className="grid gap-2">
        <div className="flex items-center gap-2">
          <Checkbox
            id="ai-enabled"
            checked={settings.ai_enabled}
            onCheckedChange={(checked) => save.mutate({ ai_enabled: checked === true })}
          />
          <Label htmlFor="ai-enabled">{t("Write activity summaries with a local AI model")}</Label>
        </div>
        <p className="text-xs text-muted-foreground">
          {t(
            "Compares an activity with earlier ones over a similar distance and with the recent weeks. New activities are summarised after import. Needs Ollama running on this computer; your data doesn't leave it.",
          )}
        </p>
      </div>
      <div className="grid gap-2">
        <Label htmlFor="ai-url">{t("Ollama server")}</Label>
        <div className="flex gap-2">
          <Input
            id="ai-url"
            value={serverUrl}
            onChange={(e) => setUrl(e.target.value)}
            onBlur={saveUrl}
          />
          <Button
            variant="outline"
            disabled={models.isPending}
            onClick={() => models.mutate(serverUrl.trim())}
          >
            {models.isPending && <Loader2 className="animate-spin" />}
            {t("Load models")}
          </Button>
        </div>
        {models.data?.length === 0 && (
          <p className="text-xs text-muted-foreground">
            {t("No models installed. Install one with “ollama pull qwen2.5:7b”.")}
          </p>
        )}
      </div>
      <div className="grid gap-2">
        <Label>{t("Model")}</Label>
        <Select
          value={settings.ai_model}
          onValueChange={(ai_model) => save.mutate({ ai_model })}
          disabled={choices.length === 0}
        >
          <SelectTrigger className="w-72">
            <SelectValue placeholder={t("Load the models first")} />
          </SelectTrigger>
          <SelectContent>
            {choices.map((name) => (
              <SelectItem key={name} value={name}>
                {name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-xs text-muted-foreground">
          {t("Models with 7 to 14 billion parameters (e.g. qwen2.5:14b) work well.")}
        </p>
      </div>
      <div className="grid gap-2">
        <Label>{t("Compare with the last")}</Label>
        <Select
          value={String(settings.ai_period_weeks)}
          onValueChange={(weeks) => save.mutate({ ai_period_weeks: Number(weeks) })}
        >
          <SelectTrigger className="w-56">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {PERIOD_WEEKS.map((weeks) => (
              <SelectItem key={weeks} value={String(weeks)}>
                {t("{{count}} week", { count: weeks })}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
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

/** Language, appearance, AI summary and storage (`ui/window_system_settings.py`). */
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
            {t("Language, appearance, activity panel, AI summary and storage")}
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
            <TabsContent value="panel">
              <PanelPage />
            </TabsContent>
            <TabsContent value="ai">
              <AiPage />
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
