import { useQuery } from "@tanstack/react-query";

import type { ViewMode } from "@/bindings/ViewMode";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatDuration, formatNumber, formatPace } from "@/lib/format";

/** Per-km (per 5 km for rides) segments, the `activity_details` rows. */
export function SegmentsTab({ activityId, view }: { activityId: number; view: ViewMode }) {
  const { data: segments = [] } = useQuery({
    queryKey: ["segments", activityId],
    queryFn: () => api.activitySegments(activityId),
  });
  const steps = view !== "Cycling";
  const head = "text-right";
  const cell = "text-right tabular-nums";
  return (
    <div className="scrollbar-visible h-full overflow-auto rounded-lg border">
      <Table>
        <TableHeader className="sticky top-0 bg-background">
          <TableRow>
            <TableHead>#</TableHead>
            <TableHead className={head}>{t("Distance (km)")}</TableHead>
            <TableHead className={head}>{t("Time")}</TableHead>
            <TableHead className={head}>{t("Pace (/km)")}</TableHead>
            <TableHead className={head}>{t("Speed (km/h)")}</TableHead>
            <TableHead className={head}>{t("HR (bpm)")}</TableHead>
            <TableHead className={head}>{t("Power (W)")}</TableHead>
            {steps && <TableHead className={head}>{t("Cadence (spm)")}</TableHead>}
            <TableHead className={head}>{t("Elevation (m)")}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {segments.map((s) => (
            <TableRow key={s.index}>
              <TableCell className="text-muted-foreground">{s.index + 1}</TableCell>
              <TableCell className={cell}>{formatNumber(s.distance, 2)}</TableCell>
              <TableCell className={cell}>{formatDuration(s.duration)}</TableCell>
              <TableCell className={cell}>{formatPace(s.pace)}</TableCell>
              <TableCell className={cell}>
                {formatNumber(s.speed == null ? null : s.speed * 3.6, 1)}
              </TableCell>
              <TableCell className={cell}>{formatNumber(s.heartRate || null)}</TableCell>
              <TableCell className={cell}>{formatNumber(s.power || null)}</TableCell>
              {steps && (
                <TableCell className={cell}>{formatNumber(s.steps ? s.steps * 2 : null)}</TableCell>
              )}
              <TableCell className={cell}>{formatNumber(s.elevationGain)}</TableCell>
            </TableRow>
          ))}
          {segments.length === 0 && (
            <TableRow>
              <TableCell colSpan={9} className="py-10 text-center text-muted-foreground">
                {t("No segments for this activity.")}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>
    </div>
  );
}
