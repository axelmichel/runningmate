import { useQuery } from "@tanstack/react-query";
import {
  ChartLine,
  Gauge,
  HeartPulse,
  LayoutDashboard,
  ListOrdered,
  Map as MapIcon,
  PanelLeft,
  Pencil,
  type LucideIcon,
} from "lucide-react";
import { useState } from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
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

const TABS = [
  ["general", "General", LayoutDashboard],
  ["segments", "Segments", ListOrdered],
  ["map", "Map", MapIcon],
  ["statistics", "Statistics", ChartLine],
  ["effect", "Effect", Gauge],
  ["zones", "Heart rate zones", HeartPulse],
  ["edit", "Edit", Pencil],
] as const satisfies readonly (readonly [string, string, LucideIcon])[];

type Tab = (typeof TABS)[number][0];

const NAV_STORAGE_KEY = "runningmate-detail-nav";

type ActivityDetailDialogProps = {
  activityId: number | null;
  onClose: () => void;
  dataDir?: string;
};

/** The detail window of the Python app (`ui/dialog_detail.py`) as a large dialog. */
export function ActivityDetailDialog({ activityId, onClose, dataDir }: ActivityDetailDialogProps) {
  return (
    <Dialog open={activityId !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="flex h-[85vh] max-w-6xl flex-col gap-4 sm:max-w-6xl">
        {activityId !== null && (
          <DetailContent key={activityId} activityId={activityId} dataDir={dataDir} />
        )}
      </DialogContent>
    </Dialog>
  );
}

function DetailContent({ activityId, dataDir }: { activityId: number; dataDir?: string }) {
  const { data: activity } = useQuery({
    queryKey: ["activity", activityId],
    queryFn: () => api.getActivity(activityId),
  });
  // Activities that were never saved on the edit page open there; edited ones on General.
  const [tab, setTab] = useState<Tab | null>(null);
  const [navOpen, setNavOpen] = useState(() => localStorage.getItem(NAV_STORAGE_KEY) !== "closed");
  const toggleNav = () =>
    setNavOpen((open) => {
      localStorage.setItem(NAV_STORAGE_KEY, open ? "closed" : "open");
      return !open;
    });
  if (!activity) {
    return (
      <>
        <DialogTitle className="sr-only">Activity</DialogTitle>
        <Skeleton className="size-full" />
      </>
    );
  }
  const current: Tab = tab ?? (activity.edited ? "general" : "edit");

  return (
    <>
      <DialogHeader className="flex-row items-center gap-3">
        <Button
          variant="ghost"
          size="icon-sm"
          aria-pressed={navOpen}
          onClick={toggleNav}
          className="shrink-0"
        >
          <PanelLeft />
          <span className="sr-only">Toggle navigation</span>
        </Button>
        {/* Title and date/type on one line; pr-8 keeps clear of the dialog's close button. */}
        <div className="flex min-w-0 items-baseline gap-3 pr-8">
          <DialogTitle className="truncate">{activity.title}</DialogTitle>
          <DialogDescription className="shrink-0 whitespace-nowrap">
            {formatDateTime(activity.date)} · {activity.typeDetail || activity.activityType}
          </DialogDescription>
        </div>
      </DialogHeader>
      <Tabs
        value={current}
        onValueChange={(value) => setTab(value as Tab)}
        orientation="vertical"
        className="min-h-0 flex-1 flex-row gap-4"
      >
        {/* Collapses to icons, like the app sidebar; labels then show as tooltips. */}
        <TabsList
          className={cn(
            "h-fit shrink-0 flex-col items-stretch gap-0.5 overflow-hidden bg-transparent p-0 transition-[width] duration-200 ease-linear",
            navOpen ? "w-48" : "w-10",
          )}
        >
          {TABS.map(([value, label, Icon]) => {
            const trigger = (
              <TabsTrigger
                key={value}
                value={value}
                aria-label={label}
                className="h-8 justify-start gap-2 overflow-hidden px-2.5 hover:bg-muted/60 data-active:bg-muted data-active:text-foreground data-active:shadow-none"
              >
                <Icon className="shrink-0" />
                <span className={cn("truncate", !navOpen && "sr-only")}>{label}</span>
              </TabsTrigger>
            );
            return navOpen ? (
              trigger
            ) : (
              <Tooltip key={value}>
                <TooltipTrigger asChild>{trigger}</TooltipTrigger>
                <TooltipContent side="right">{label}</TooltipContent>
              </Tooltip>
            );
          })}
        </TabsList>
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
            <EditTab activity={activity} onSaved={() => setTab("general")} />
          </TabsContent>
        </div>
      </Tabs>
    </>
  );
}
