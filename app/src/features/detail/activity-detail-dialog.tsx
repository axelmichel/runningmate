import { useQuery } from "@tanstack/react-query";
import {
  ChartLine,
  ChevronLeft,
  ChevronRight,
  Gauge,
  HeartPulse,
  LayoutDashboard,
  ListOrdered,
  Map as MapIcon,
  Maximize2,
  Minimize2,
  PanelLeft,
  Pencil,
  X,
} from "lucide-react";
import { useState } from "react";

import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { NavTabs, type NavItem } from "@/components/nav-tabs";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { formatDateTime } from "@/lib/format";
import { cn } from "@/lib/utils";

import { EditTab } from "./edit-tab";
import { EffectTab } from "./effect-tab";
import { GeneralTab } from "./general-tab";
import { MapTab } from "./map-tab";
import { SegmentsTab } from "./segments-tab";
import { StatsTab } from "./stats-tab";
import { ZonesTab } from "./zones-tab";

type Tab = "general" | "segments" | "map" | "statistics" | "effect" | "zones" | "edit";

const NAV_ITEMS: (NavItem & { value: Tab })[] = [
  { value: "general", label: tk("General"), icon: LayoutDashboard },
  { value: "segments", label: tk("Segments"), icon: ListOrdered },
  { value: "map", label: tk("Map"), icon: MapIcon },
  { value: "statistics", label: tk("Statistics"), icon: ChartLine },
  { value: "effect", label: tk("Effect"), icon: Gauge },
  { value: "zones", label: tk("Heart rate zones"), icon: HeartPulse },
  { value: "edit", label: tk("Edit"), icon: Pencil },
];

const NAV_STORAGE_KEY = "runningmate-detail-nav";
const MAXIMIZED_STORAGE_KEY = "runningmate-detail-maximized";

/** A localStorage-backed on/off switch. */
function useStoredToggle(key: string, initial: boolean) {
  const [on, setOn] = useState(() => {
    const stored = localStorage.getItem(key);
    // "open" is the value earlier versions stored for the navigation toggle.
    return stored === null ? initial : stored === "on" || stored === "open";
  });
  const toggle = () =>
    setOn((value) => {
      localStorage.setItem(key, value ? "off" : "on");
      return !value;
    });
  return [on, toggle] as const;
}

type ActivityDetailDialogProps = {
  activityId: number | null;
  onClose: () => void;
  /** Move to the previous/next activity in the table order; omitted at the ends. */
  onPrevious?: () => void;
  onNext?: () => void;
  dataDir?: string;
};

/** The detail window of the Python app (`ui/dialog_detail.py`) as a large dialog. */
export function ActivityDetailDialog({
  activityId,
  onClose,
  onPrevious,
  onNext,
  dataDir,
}: ActivityDetailDialogProps) {
  // The page the user picked stays selected while moving between activities.
  const [tab, setTab] = useState<Tab | null>(null);
  const [navOpen, toggleNav] = useStoredToggle(NAV_STORAGE_KEY, true);
  const [maximized, toggleMaximized] = useStoredToggle(MAXIMIZED_STORAGE_KEY, false);

  return (
    <Dialog
      open={activityId !== null}
      onOpenChange={(open) => {
        if (open) return;
        setTab(null);
        onClose();
      }}
    >
      <DialogContent
        // The close button sits in the header row, so it lines up with the other controls.
        showCloseButton={false}
        // Maximized, the dialog fills the window below the app title bar; the backdrop still
        // covers the bar, so it is tinted and blurred as with the smaller dialog.
        className={cn(
          "flex flex-col gap-4",
          maximized
            ? "top-(--titlebar-height) left-0 h-[calc(100svh-var(--titlebar-height))] w-screen max-w-none translate-x-0 translate-y-0 rounded-none ring-0 sm:max-w-none"
            : "h-[85vh] max-w-6xl sm:max-w-6xl",
        )}
      >
        {activityId !== null && (
          <DetailContent
            key={activityId}
            activityId={activityId}
            dataDir={dataDir}
            tab={tab}
            onTabChange={setTab}
            navOpen={navOpen}
            onToggleNav={toggleNav}
            maximized={maximized}
            onToggleMaximized={toggleMaximized}
            onPrevious={onPrevious}
            onNext={onNext}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}

type DetailContentProps = {
  activityId: number;
  dataDir?: string;
  tab: Tab | null;
  onTabChange: (tab: Tab) => void;
  navOpen: boolean;
  onToggleNav: () => void;
  maximized: boolean;
  onToggleMaximized: () => void;
  onPrevious?: () => void;
  onNext?: () => void;
};

/** Icon button for the header; extra props pass through (e.g. from `DialogClose asChild`). */
function HeaderButton({
  label,
  pressed,
  ...props
}: React.ComponentProps<typeof Button> & { label: string; pressed?: boolean }) {
  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      title={label}
      aria-pressed={pressed}
      className="shrink-0"
      {...props}
    />
  );
}

function DetailContent({
  activityId,
  dataDir,
  tab,
  onTabChange,
  navOpen,
  onToggleNav,
  maximized,
  onToggleMaximized,
  onPrevious,
  onNext,
}: DetailContentProps) {
  const { data: activity } = useQuery({
    queryKey: ["activity", activityId],
    queryFn: () => api.getActivity(activityId),
  });
  if (!activity) {
    return (
      <>
        <DialogTitle className="sr-only">{t("Activity")}</DialogTitle>
        <Skeleton className="size-full" />
      </>
    );
  }
  // Unreviewed activities (no edit, comment or media yet) open on the edit page.
  const current: Tab = tab ?? (activity.reviewed ? "general" : "edit");

  return (
    <>
      <DialogHeader className="flex-row items-center gap-3">
        <HeaderButton label={t("Toggle navigation")} pressed={navOpen} onClick={onToggleNav}>
          <PanelLeft />
        </HeaderButton>
        <div className="flex min-w-0 flex-1 items-baseline gap-3">
          <DialogTitle className="truncate">{activity.title}</DialogTitle>
          <DialogDescription className="shrink-0 whitespace-nowrap">
            {formatDateTime(activity.date)} ·{" "}
            {t(activity.typeDetail || activity.activityType || "")}
          </DialogDescription>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <HeaderButton label={t("Previous activity")} onClick={onPrevious} disabled={!onPrevious}>
            <ChevronLeft />
          </HeaderButton>
          <HeaderButton label={t("Next activity")} onClick={onNext} disabled={!onNext}>
            <ChevronRight />
          </HeaderButton>
          <HeaderButton
            label={maximized ? t("Restore size") : t("Maximize")}
            pressed={maximized}
            onClick={onToggleMaximized}
          >
            {maximized ? <Minimize2 /> : <Maximize2 />}
          </HeaderButton>
          <DialogClose asChild>
            <HeaderButton label={t("Close")}>
              <X />
            </HeaderButton>
          </DialogClose>
        </div>
      </DialogHeader>
      <Tabs
        value={current}
        onValueChange={(value) => onTabChange(value as Tab)}
        orientation="vertical"
        className="min-h-0 flex-1 flex-row gap-4"
      >
        <NavTabs items={NAV_ITEMS} open={navOpen} />
        <div className="scrollbar-visible min-h-0 min-w-0 flex-1 overflow-auto pr-1">
          <TabsContent value="general">
            <GeneralTab activity={activity} dataDir={dataDir} />
          </TabsContent>
          <TabsContent value="segments" className="h-full">
            <SegmentsTab activityId={activity.id} view={activity.viewMode} />
          </TabsContent>
          <TabsContent value="map" className="h-full">
            <MapTab activityId={activity.id} view={activity.viewMode} />
          </TabsContent>
          <TabsContent value="statistics">
            <StatsTab activityId={activity.id} />
          </TabsContent>
          <TabsContent value="effect">
            <EffectTab activityId={activity.id} />
          </TabsContent>
          <TabsContent value="zones">
            <ZonesTab activityId={activity.id} />
          </TabsContent>
          <TabsContent value="edit">
            <EditTab activity={activity} onSaved={() => onTabChange("general")} />
          </TabsContent>
        </div>
      </Tabs>
    </>
  );
}
