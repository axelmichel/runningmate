import { useMemo } from "react";
import { CartesianGrid, ComposedChart, Line, ReferenceLine, XAxis, YAxis } from "recharts";

import type { RankedEffort } from "@/bindings/RankedEffort";
import type { ViewMode } from "@/bindings/ViewMode";
import { ChartContainer, ChartTooltip, type ChartConfig } from "@/components/ui/chart";
import { t } from "@/i18n";
import { formatDate, formatDuration } from "@/lib/format";

import { formatEffortSpeed, TROPHY_COLORS, TROPHY_RANKS } from "./segment-math";

type Pass = RankedEffort & { latest: boolean };

const chartConfig = () =>
  ({
    elapsed: { label: t("Time"), color: "var(--chart-1)" },
  }) satisfies ChartConfig;

/** "M:SS" below an hour, "H:MM:SS" above, for the axis. */
function shortDuration(seconds: number) {
  const text = formatDuration(seconds);
  return seconds < 3600 ? text.replace(/^0:0?/, "") : text;
}

function Dot({
  cx,
  cy,
  payload,
  onOpen,
}: {
  cx?: number;
  cy?: number;
  payload?: Pass;
  onOpen?: (activityId: number) => void;
}) {
  if (cx == null || cy == null || !payload) return null;
  const medal = payload.rank <= TROPHY_RANKS ? TROPHY_COLORS[payload.rank - 1] : undefined;
  return (
    <circle
      cx={cx}
      cy={cy}
      r={payload.latest ? 6 : medal ? 5 : 3.5}
      fill={medal ?? "var(--color-elapsed)"}
      stroke={payload.latest ? "var(--foreground)" : "var(--background)"}
      strokeWidth={payload.latest ? 2 : 1}
      style={onOpen ? { cursor: "pointer" } : undefined}
      onClick={onOpen ? () => onOpen(payload.activityId) : undefined}
    />
  );
}

/** Every pass over a segment by date, the time on the vertical axis. */
export function SegmentProgressChart({
  efforts,
  view,
  distance,
  onOpen,
}: {
  efforts: RankedEffort[];
  view: ViewMode;
  /** Segment length, m. */
  distance: number;
  onOpen?: (activityId: number) => void;
}) {
  const data = useMemo<Pass[]>(() => {
    const byDate = [...efforts].sort((a, b) => a.date - b.date);
    return byDate.map((e, i) => ({ ...e, latest: i === byDate.length - 1 }));
  }, [efforts]);
  if (data.length === 0) {
    return (
      <p className="py-8 text-center text-sm text-muted-foreground">
        {t("No passes over this segment yet.")}
      </p>
    );
  }
  const best = Math.min(...data.map((e) => e.elapsed));

  return (
    <div className="grid gap-2">
      <ChartContainer config={chartConfig()} className="aspect-auto h-80 w-full">
        <ComposedChart data={data} margin={{ top: 16, right: 16 }}>
          <CartesianGrid vertical={false} />
          <XAxis
            dataKey="date"
            type="number"
            scale="time"
            domain={["dataMin", "dataMax"]}
            padding={{ left: 16, right: 16 }}
            tickFormatter={(v: number) => formatDate(v)}
            tickLine={false}
            axisLine={false}
            minTickGap={24}
          />
          <YAxis
            dataKey="elapsed"
            width={52}
            tickLine={false}
            axisLine={false}
            domain={[(min: number) => Math.max(0, min * 0.95), (max: number) => max * 1.05]}
            tickFormatter={(v: number) => shortDuration(v)}
          />
          <ReferenceLine
            y={best}
            stroke={TROPHY_COLORS[0]}
            strokeDasharray="4 4"
            label={{
              value: `${t("Best")} ${shortDuration(best)}`,
              position: "insideTopLeft",
              fontSize: 10,
            }}
          />
          <ChartTooltip
            content={({ active, payload }) => {
              const pass = payload?.[0]?.payload as Pass | undefined;
              if (!active || !pass) return null;
              return (
                <div className="grid gap-0.5 rounded-lg border bg-background px-2.5 py-1.5 text-xs shadow-xl">
                  <div className="font-medium">{formatDate(pass.date)}</div>
                  <div data-private className="text-muted-foreground">
                    {pass.activityTitle}
                  </div>
                  <div className="tabular-nums">
                    {formatDuration(pass.elapsed)} ·{" "}
                    {formatEffortSpeed(view, distance, pass.elapsed)}
                  </div>
                  <div className="text-muted-foreground tabular-nums">
                    {t("{{rank}} of {{tries}}", { rank: pass.rank, tries: data.length })}
                  </div>
                </div>
              );
            }}
          />
          <Line
            dataKey="elapsed"
            type="linear"
            stroke="var(--color-elapsed)"
            strokeOpacity={0.35}
            strokeWidth={1.5}
            dot={(props) => <Dot key={props.index} {...props} onOpen={onOpen} />}
            activeDot={false}
            isAnimationActive={false}
          />
        </ComposedChart>
      </ChartContainer>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {TROPHY_COLORS.map((color, i) => (
          <span key={color} className="flex items-center gap-1.5">
            <span className="size-2.5 rounded-full" style={{ background: color }} />
            {t("Place {{rank}}", { rank: i + 1 })}
          </span>
        ))}
        <span className="flex items-center gap-1.5">
          <span className="size-2.5 rounded-full border-2 border-foreground" />
          {t("Latest")}
        </span>
        {onOpen && <span>{t("Click a point to open the activity.")}</span>}
      </div>
    </div>
  );
}
