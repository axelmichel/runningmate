import type { ZoneTime } from "@/bindings/ZoneTime";
import { t } from "@/i18n";
import { formatDuration, formatNumber } from "@/lib/format";

/** Zone colours from dark to light, after `ui/widget_heart_rate_zones.py`. */
const ZONE_COLORS = ["#35193e", "#701f57", "#ad1759", "#f37651", "#f6b48f"];

/**
 * One bar per heart-rate zone with its share and time. All rows share one grid, so the label
 * and time columns fit their longest entry (without wrapping) and the bars stay aligned.
 */
export function ZoneBars({ zones }: { zones: ZoneTime[] }) {
  return (
    <div className="grid grid-cols-[auto_1fr_auto] items-center gap-x-3 gap-y-2">
      {zones.map((zone) => (
        <div key={zone.zone} className="contents">
          <span className="whitespace-nowrap">
            {t("Zone {{zone}}", { zone: zone.zone })}{" "}
            <span className="text-muted-foreground">
              ({zone.minBpm}–{zone.maxBpm})
            </span>
          </span>
          <div className="h-2.5 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full"
              style={{ width: `${zone.percent}%`, backgroundColor: ZONE_COLORS[zone.zone - 1] }}
            />
          </div>
          <span className="text-right whitespace-nowrap tabular-nums">
            {formatNumber(zone.percent)} % · {formatDuration(zone.seconds).replace(/^0:/, "")}
          </span>
        </div>
      ))}
    </div>
  );
}
