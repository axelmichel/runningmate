import { t } from "@/i18n";
import { formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { position, segmentOf, type Scale } from "./scales";

/**
 * A horizontal scale in coloured segments with a marker at `value`, like the VO2max and
 * training-status scales of Garmin Connect and Apple Health.
 */
export function ScaleBar({
  scale,
  value,
  format = (v) => formatNumber(v, 1),
}: {
  scale: Scale;
  value: number;
  format?: (value: number) => string;
}) {
  const current = segmentOf(scale, value);
  const at = position(scale, value);
  return (
    <div className="grid gap-1">
      <div className="relative h-6">
        <span
          className="absolute bottom-0 -translate-x-1/2 text-xs font-semibold whitespace-nowrap tabular-nums"
          style={{ left: `${Math.min(95, Math.max(5, at))}%` }}
        >
          {format(value)}
        </span>
      </div>
      <div className="relative">
        <div className="flex h-2.5 overflow-hidden rounded-full">
          {scale.segments.map((segment) => (
            <div
              key={segment.label}
              className={cn("h-full", segment !== current && "opacity-35")}
              style={{
                width: `${((segment.to - segment.from) / (scale.max - scale.min)) * 100}%`,
                backgroundColor: segment.color,
              }}
            />
          ))}
        </div>
        <div
          className="absolute top-1/2 size-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-background shadow"
          style={{ left: `${at}%`, backgroundColor: current.color }}
          aria-hidden
        />
      </div>
      <div className="flex text-[10px] text-muted-foreground">
        {scale.segments.map((segment) => (
          <span
            key={segment.label}
            className={cn(
              "truncate text-center",
              segment === current && "font-medium text-foreground",
            )}
            style={{ width: `${((segment.to - segment.from) / (scale.max - scale.min)) * 100}%` }}
          >
            {t(segment.label)}
          </span>
        ))}
      </div>
    </div>
  );
}
