import type { ActivityDetail } from "@/bindings/ActivityDetail";
import { ActivityPanel } from "@/features/activities/activity-panel";
import { t } from "@/i18n";
import { formatMinSec, formatNumber } from "@/lib/format";

import { MediaGallery } from "./media-gallery";

function Fact({ label, value }: { label: string; value: string | null }) {
  if (!value || value === "–") return null;
  return (
    <div className="flex justify-between gap-4 border-b py-1.5 text-sm last:border-0">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium tabular-nums">{value}</span>
    </div>
  );
}

/** Overview, media, comment and the per-type metrics (`widget_run_details.py` and friends). */
export function GeneralTab({ activity, dataDir }: { activity: ActivityDetail; dataDir?: string }) {
  const steps = activity.viewMode !== "Cycling";
  return (
    <div className="grid gap-6 lg:grid-cols-[20rem_1fr]">
      <ActivityPanel activityId={activity.id} dataDir={dataDir} />
      <div className="grid content-start gap-6">
        <MediaGallery activityId={activity.id} />
        {activity.comment && <p className="text-sm whitespace-pre-wrap">{activity.comment}</p>}
        <div>
          <Fact
            label={t("Calories")}
            value={activity.calories ? `${formatNumber(activity.calories)} kcal` : null}
          />
          <Fact
            label={t("Average heart rate")}
            value={activity.avgHeartRate ? `${activity.avgHeartRate} bpm` : null}
          />
          <Fact
            label={t("Average power")}
            value={activity.avgPower ? `${activity.avgPower} W` : null}
          />
          <Fact
            label={t("Average speed")}
            value={activity.avgSpeed ? `${formatNumber(activity.avgSpeed, 1)} km/h` : null}
          />
          <Fact label={t("Fastest pace")} value={`${formatMinSec(activity.fastestPace)} /km`} />
          <Fact label={t("Slowest pace")} value={`${formatMinSec(activity.slowestPace)} /km`} />
          {steps && (
            <Fact
              label={t("Cadence")}
              value={activity.avgSteps ? `${activity.avgSteps} spm` : null}
            />
          )}
          {steps && (
            <Fact
              label={t("Steps")}
              value={activity.totalSteps ? formatNumber(activity.totalSteps) : null}
            />
          )}
          <Fact
            label={t("Pause")}
            value={
              activity.pause && activity.pause !== "00:00" ? formatMinSec(activity.pause) : null
            }
          />
          <Fact
            label={activity.viewMode === "Cycling" ? t("Bike") : t("Shoes")}
            value={activity.equipment?.name ?? null}
          />
        </div>
      </div>
    </div>
  );
}
