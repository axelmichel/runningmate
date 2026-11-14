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
} from "lucide-react";
import { useState } from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { NavTabs, type NavItem } from "@/components/nav-tabs";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { api } from "@/lib/api";
import { formatDateTime } from "@/lib/format";

import { EditTab } from "./edit-tab";
import { EffectTab } from "./effect-tab";
import { GeneralTab } from "./general-tab";
import { MapTab } from "./map-tab";
import { SegmentsTab } from "./segments-tab";
import { StatsTab } from "./stats-tab";
import { ZonesTab } from "./zones-tab";

type Tab = "general" | "segments" | "map" | "statistics" | "effect" | "zones" | "edit";

const NAV_ITEMS: (NavItem & { value: Tab })[] = [
  { value: "general", label: "General", icon: LayoutDashboard },
  { value: "segments", label: "Segments", icon: ListOrdered },
  { value: "map", label: "Map", icon: MapIcon },
  { value: "statistics", label: "Statistics", icon: ChartLine },
  { value: "effect", label: "Effect", icon: Gauge },
  { value: "zones", label: "Heart rate zones", icon: HeartPulse },
  { value: "edit", label: "Edit", icon: Pencil },
];

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
  // Unreviewed activities (no edit, comment or media yet) open on the edit page.
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
  const current: Tab = tab ?? (activity.reviewed ? "general" : "edit");

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
            <EditTab activity={activity} onSaved={() => setTab("general")} />
          </TabsContent>
        </div>
      </Tabs>
    </>
  );
}
