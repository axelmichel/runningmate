import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Check, Pencil, Trash2, X } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { RouteSegment } from "@/bindings/RouteSegment";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { VIEW_MODES } from "@/features/activities/view-mode";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDate, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { Leaderboard } from "./leaderboard";
import { formatGrade, formatSegmentDistance, invalidateSegments } from "./segment-math";
import { SegmentHighlights } from "./segment-highlights";
import { SegmentMap } from "./segment-map";
import { SegmentProfile } from "./segment-profile";
import { SegmentProgressChart } from "./segment-progress-chart";

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-lg font-semibold text-primary tabular-nums">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}

/** Distance, grade, climb and tries of a segment. */
export function SegmentStats({ segment }: { segment: RouteSegment }) {
  return (
    <div className="grid grid-cols-2 gap-x-6 gap-y-3 sm:grid-cols-3">
      <Stat label={t("Distance")} value={formatSegmentDistance(segment.distance)} />
      <Stat label={t("Average grade")} value={formatGrade(segment.avgGrade)} />
      <Stat label={t("Max grade")} value={formatGrade(segment.maxGrade)} />
      <Stat label={t("Elevation gain")} value={`${formatNumber(segment.elevationGain)} m`} />
      <Stat
        label={t("Lowest / highest")}
        value={`${formatNumber(segment.minElevation)}–${formatNumber(segment.maxElevation)} m`}
      />
      <Stat
        label={t("Tries")}
        value={
          segment.activities === segment.tries
            ? formatNumber(segment.tries)
            : `${formatNumber(segment.tries)} (${t("{{count}} activities", { count: segment.activities })})`
        }
      />
    </div>
  );
}

function Title({ segment }: { segment: RouteSegment }) {
  const queryClient = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState(segment.title);
  const rename = useMutation({
    mutationFn: () => api.renameSegment(segment.id, title),
    onSuccess: () => {
      setEditing(false);
      invalidateSegments(queryClient);
    },
    onError: (error) =>
      toast.error(t("Could not rename the segment: {{error}}", { error: String(error) })),
  });
  if (!editing) {
    return (
      <div className="flex min-w-0 items-center gap-1">
        <DialogTitle className="truncate">{segment.title}</DialogTitle>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("Rename")}
          title={t("Rename")}
          onClick={() => {
            setTitle(segment.title);
            setEditing(true);
          }}
        >
          <Pencil />
        </Button>
      </div>
    );
  }
  return (
    <form
      className="flex min-w-0 flex-1 items-center gap-1"
      onSubmit={(event) => {
        event.preventDefault();
        if (title.trim()) rename.mutate();
      }}
    >
      <DialogTitle className="sr-only">{segment.title}</DialogTitle>
      <Input
        autoFocus
        value={title}
        aria-label={t("Title")}
        onChange={(event) => setTitle(event.target.value)}
        className="max-w-sm"
      />
      <Button
        type="submit"
        variant="ghost"
        size="icon-sm"
        aria-label={t("Save")}
        title={t("Save")}
        disabled={!title.trim() || rename.isPending}
      >
        <Check />
      </Button>
      <Button
        type="button"
        variant="ghost"
        size="icon-sm"
        aria-label={t("Cancel")}
        title={t("Cancel")}
        onClick={() => setEditing(false)}
      >
        <X />
      </Button>
    </form>
  );
}

function Content({
  segmentId,
  highlight,
  onOpenActivity,
  onDeleted,
}: {
  segmentId: number;
  highlight?: number;
  onOpenActivity?: (activityId: number) => void;
  onDeleted: () => void;
}) {
  const queryClient = useQueryClient();
  const [confirming, setConfirming] = useState(false);
  const { data: detail } = useQuery({
    queryKey: ["routeSegment", segmentId],
    queryFn: () => api.getSegment(segmentId),
  });
  const remove = useMutation({
    mutationFn: () => api.deleteSegment(segmentId),
    onSuccess: () => {
      toast.success(t("Segment deleted"));
      onDeleted();
      invalidateSegments(queryClient);
    },
    onError: (error) =>
      toast.error(t("Could not delete the segment: {{error}}", { error: String(error) })),
  });
  if (!detail) {
    return (
      <>
        <DialogTitle className="sr-only">{t("Segment")}</DialogTitle>
        <Skeleton className="h-96 w-full" />
      </>
    );
  }
  const { segment, leaderboard } = detail;
  const sport = VIEW_MODES.find((v) => v.mode === segment.view);
  return (
    <>
      <DialogHeader className="flex-row items-center gap-3">
        <div className="flex min-w-0 flex-1 items-baseline gap-3">
          <Title key={segment.title} segment={segment} />
          <DialogDescription className="shrink-0 whitespace-nowrap">
            {sport ? t(sport.label) : ""} ·{" "}
            {t("created {{date}}", { date: formatDate(segment.created) })}
          </DialogDescription>
        </div>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t("Delete segment")}
          title={t("Delete segment")}
          onClick={() => setConfirming(true)}
        >
          <Trash2 />
        </Button>
        <DialogClose asChild>
          <Button variant="ghost" size="icon-sm" aria-label={t("Close")} title={t("Close")}>
            <X />
          </Button>
        </DialogClose>
      </DialogHeader>
      <Tabs defaultValue="overview" className="min-h-0 flex-1 gap-3">
        <TabsList>
          <TabsTrigger value="overview">{t("Overview")}</TabsTrigger>
          <TabsTrigger value="progress">{t("Progress")}</TabsTrigger>
        </TabsList>
        {/* The top stays in place and the ranking scrolls below it; the whole page only
            scrolls when the window is too low for both. */}
        <TabsContent
          value="overview"
          className="scrollbar-visible -m-1 flex min-h-0 flex-col gap-4 overflow-auto p-1"
        >
          <div className="grid shrink-0 gap-4 md:grid-cols-[3fr_2fr]">
            <div className="h-72">
              <SegmentMap points={segment.points} />
            </div>
            <div className="grid content-start gap-4">
              <SegmentStats segment={segment} />
              <SegmentProfile points={segment.points} className="h-40" />
            </div>
          </div>
          <SegmentHighlights
            segment={segment}
            onOpen={onOpenActivity}
            className="shrink-0 text-sm"
          />
          <div className="flex min-h-64 flex-1 flex-col">
            <Leaderboard
              efforts={leaderboard}
              view={segment.view}
              distance={segment.distance}
              highlight={highlight}
              onOpen={onOpenActivity}
            />
          </div>
        </TabsContent>
        <TabsContent value="progress" className="min-h-0 overflow-auto">
          <SegmentProgressChart
            efforts={leaderboard}
            view={segment.view}
            distance={segment.distance}
            onOpen={onOpenActivity}
          />
        </TabsContent>
      </Tabs>
      <AlertDialog open={confirming} onOpenChange={setConfirming}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Delete this segment?")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("The segment and its ranking are removed; the activities stay as they are.")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction variant="destructive" onClick={() => remove.mutate()}>
              {t("Delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

/** A segment with map, profile, numbers and every pass, fastest first. */
export function SegmentDetailDialog({
  segmentId,
  onClose,
  highlight,
  onOpenActivity,
  covered = false,
}: {
  segmentId: number | null;
  onClose: () => void;
  /** Activity whose passes are marked in the ranking. */
  highlight?: number;
  /** Makes the ranking rows open their activity. */
  onOpenActivity?: (activityId: number) => void;
  /** Another dialog is open on top: WebKit would paint the map's tiles over it. */
  covered?: boolean;
}) {
  return (
    <Dialog open={segmentId !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        showCloseButton={false}
        className={cn(
          "flex h-[85vh] max-w-5xl flex-col gap-4 sm:max-w-5xl",
          covered && "[&_.leaflet-container]:invisible",
        )}
      >
        {segmentId !== null && (
          <Content
            key={segmentId}
            segmentId={segmentId}
            highlight={highlight}
            onOpenActivity={onOpenActivity}
            onDeleted={onClose}
          />
        )}
      </DialogContent>
    </Dialog>
  );
}
