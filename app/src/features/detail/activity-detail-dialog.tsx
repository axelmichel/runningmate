import { useQuery } from "@tanstack/react-query";
import { useState } from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { api } from "@/lib/api";
import { formatDateTime } from "@/lib/format";

import { EditTab } from "./edit-tab";
import { EffectTab } from "./effect-tab";
import { GeneralTab } from "./general-tab";
import { MapTab } from "./map-tab";
import { SegmentsTab } from "./segments-tab";
import { StatsTab } from "./stats-tab";
import { ZonesTab } from "./zones-tab";

const TABS = [
  ["general", "General"],
  ["segments", "Segments"],
  ["map", "Map"],
  ["statistics", "Statistics"],
  ["effect", "Effect"],
  ["zones", "Heart rate zones"],
  ["edit", "Edit"],
] as const;

type Tab = (typeof TABS)[number][0];

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
  // New (unreviewed) imports open on the edit page, as in Python.
  const [tab, setTab] = useState<Tab | null>(null);
  if (!activity) {
    return (
      <>
        <DialogTitle className="sr-only">Activity</DialogTitle>
        <Skeleton className="size-full" />
      </>
    );
  }
  const current: Tab = tab ?? (activity.isNew ? "edit" : "general");

  return (
    <>
      <DialogHeader>
        <DialogTitle>{activity.title}</DialogTitle>
        <DialogDescription>
          {formatDateTime(activity.date)} · {activity.typeDetail || activity.activityType}
        </DialogDescription>
      </DialogHeader>
      <Tabs
        value={current}
        onValueChange={(value) => setTab(value as Tab)}
        orientation="vertical"
        className="min-h-0 flex-1 flex-row gap-4"
      >
        <TabsList className="h-fit w-44 shrink-0 flex-col items-stretch">
          {TABS.map(([value, label]) => (
            <TabsTrigger key={value} value={value} className="justify-start">
              {label}
            </TabsTrigger>
          ))}
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
