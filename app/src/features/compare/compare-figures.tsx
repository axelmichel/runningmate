import type { ZoneTime } from "@/bindings/ZoneTime";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { formatRecovery } from "@/features/detail/training-effect";
import { ZONE_COLORS } from "@/features/detail/zone-gradient";
import { t, tk } from "@/i18n";
import { formatDuration, formatNumber, formatPace } from "@/lib/format";
import { cn } from "@/lib/utils";

import {
  figuresFromDetail,
  figuresFromPoints,
  figureValue,
  sliceStretch,
  visibleFigures,
  type FigureKey,
} from "./compare-math";
import type { CompareEntry } from "./use-compare-data";

/** Colour swatch of an activity, beside its name wherever it is listed. */
export function Swatch({ color, className }: { color: string; className?: string }) {
  return (
    <span
      aria-hidden
      className={cn("inline-block size-2.5 shrink-0 rounded-full", className)}
      style={{ backgroundColor: color }}
    />
  );
}

/** Which colour is which activity: date and title, opening the activity on click. */
export function EntryLegend({
  entries,
  onOpen,
}: {
  entries: CompareEntry[];
  onOpen: (id: number) => void;
}) {
  return (
    <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs">
      {entries.map((entry) => (
        <li key={entry.id} className="min-w-0">
          <button
            type="button"
            title={t("Open details")}
            className="flex max-w-64 items-center gap-1.5 rounded-sm hover:underline focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none"
            onClick={() => onOpen(entry.id)}
          >
            <span
              aria-hidden
              className="h-0.5 w-3 shrink-0 rounded-full"
              style={{ backgroundColor: entry.color }}
            />
            <span className="shrink-0 text-muted-foreground">{entry.label}</span>
            {entry.detail?.title && (
              <span data-private className="truncate">
                {entry.detail.title}
              </span>
            )}
          </button>
        </li>
      ))}
    </ul>
  );
}

export type BarValue = { entry: CompareEntry; value: number | null };

/**
 * One metric as a row per activity: its colour, date, a bar against the largest value (or
 * `max`) and the value. Activities without the metric show a dash.
 */
export function MetricBars({
  title,
  values,
  format,
  max,
}: {
  title: string;
  values: BarValue[];
  format: (value: number) => string;
  max?: number;
}) {
  const top = max ?? Math.max(0, ...values.map((v) => v.value ?? 0));
  return (
    <div className="grid content-start gap-1.5">
      <div className="text-xs font-medium text-muted-foreground">{title}</div>
      <div className="grid grid-cols-[auto_1fr_auto] items-center gap-x-2 gap-y-1 text-xs">
        {values.map(({ entry, value }) => (
          <div key={entry.id} className="contents">
            <span className="flex items-center gap-1.5 whitespace-nowrap text-muted-foreground">
              <Swatch color={entry.color} />
              {entry.label}
            </span>
            <div className="h-2 overflow-hidden rounded-full bg-muted">
              {value != null && top > 0 && (
                <div
                  className="h-full rounded-full"
                  style={{
                    width: `${Math.min(100, (value / top) * 100)}%`,
                    backgroundColor: entry.color,
                  }}
                />
              )}
            </div>
            <span className="text-right whitespace-nowrap tabular-nums">
              {value == null ? "–" : format(value)}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

const FIGURES: Record<FigureKey, { label: string; format: (value: number) => string }> = {
  distance: { label: tk("Distance"), format: (v) => `${formatNumber(v, 2)} km` },
  duration: { label: tk("Duration"), format: (v) => formatDuration(v) },
  pace: { label: tk("Average pace"), format: (v) => `${formatPace(v)} /km` },
  speed: { label: tk("Average speed"), format: (v) => `${formatNumber(v, 1)} km/h` },
  gap: { label: tk("Grade-adjusted pace (GAP)"), format: (v) => `${formatPace(v)} /km` },
  heartRate: { label: tk("Average heart rate"), format: (v) => `${formatNumber(v)} bpm` },
  elevationGain: { label: tk("Elevation gain"), format: (v) => `${formatNumber(v)} m` },
  calories: { label: tk("Calories"), format: (v) => `${formatNumber(v)} kcal` },
  cadence: { label: tk("Steps per minute"), format: (v) => `${formatNumber(v)} spm` },
};

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card size="sm" className="overflow-visible">
      <CardHeader>
        <CardTitle className="text-sm">{title}</CardTitle>
      </CardHeader>
      <CardContent className="grid gap-4">{children}</CardContent>
    </Card>
  );
}

const GRID = "grid gap-x-8 gap-y-5 md:grid-cols-2 xl:grid-cols-3";

/**
 * Distance, time, pace or speed, GAP, heart rate, climb, calories and steps side by side; on a
 * segment's stretches they are measured on the track (no calories then).
 */
export function KeyFigures({
  entries,
  stretched,
}: {
  entries: CompareEntry[];
  stretched: boolean;
}) {
  const rows = entries.map((entry) => ({
    entry,
    view: entry.view,
    figures:
      stretched && entry.stretch
        ? figuresFromPoints(sliceStretch(entry.series ?? [], entry.stretch))
        : entry.detail
          ? figuresFromDetail(entry.detail)
          : figuresFromPoints([]),
  }));
  const keys = visibleFigures(rows);
  if (!keys.length) return null;
  return (
    <Section title={stretched ? t("Key figures on the segment") : t("Key figures")}>
      <div className={GRID}>
        {keys.map((key) => (
          <MetricBars
            key={key}
            title={t(FIGURES[key].label)}
            values={rows.map((row) => ({ entry: row.entry, value: figureValue(row, key) }))}
            format={FIGURES[key].format}
          />
        ))}
      </div>
    </Section>
  );
}

/** Aerobic and anaerobic training effect, peak EPOC, recovery and the 1–12 effect score. */
export function EffectFigures({ entries }: { entries: CompareEntry[] }) {
  if (!entries.some((e) => e.effect || e.score)) return null;
  const effect = (pick: (e: NonNullable<CompareEntry["effect"]>) => number) =>
    entries.map((entry) => ({ entry, value: entry.effect ? pick(entry.effect) : null }));
  const one = (v: number) => formatNumber(v, 1);
  return (
    <Section title={t("Training effect")}>
      <div className={GRID}>
        <MetricBars title={t("Aerobic")} values={effect((e) => e.aerobic)} format={one} max={5} />
        <MetricBars
          title={t("Anaerobic")}
          values={effect((e) => e.anaerobic)}
          format={one}
          max={5}
        />
        <MetricBars
          title={t("Peak EPOC")}
          values={effect((e) => e.peakEpoc)}
          format={(v) => t("{{value}} ml/kg", { value: formatNumber(v) })}
        />
        <MetricBars
          title={t("Recovery time")}
          values={effect((e) => e.recoveryHours)}
          format={formatRecovery}
        />
        <MetricBars
          title={t("Effect score")}
          values={entries.map((entry) => ({ entry, value: entry.score?.score ?? null }))}
          format={(v) => `${v} / 12`}
          max={12}
        />
      </div>
      {entries.some((e) => e.effect === null) && (
        <p className="text-xs text-muted-foreground">
          {t("Activities without a recorded heart rate have no training effect.")}
        </p>
      )}
    </Section>
  );
}

function ZoneStack({ zones }: { zones: ZoneTime[] }) {
  return (
    <div className="flex h-3 gap-0.5 overflow-hidden rounded-full">
      {zones
        .filter((zone) => zone.percent > 0)
        .map((zone) => (
          <div
            key={zone.zone}
            className="h-full first:rounded-l-full last:rounded-r-full"
            title={`${t("Zone {{zone}}", { zone: zone.zone })}: ${formatNumber(zone.percent)} % · ${formatDuration(zone.seconds)}`}
            style={{ flexGrow: zone.percent, backgroundColor: ZONE_COLORS[zone.zone - 1] }}
          />
        ))}
    </div>
  );
}

/** Share of time in each heart-rate zone, one stacked bar per activity. */
export function ZoneFigures({ entries }: { entries: CompareEntry[] }) {
  if (!entries.some((e) => e.zones !== undefined)) return null;
  const any = entries.some((e) => e.zones?.some((zone) => zone.seconds > 0));
  return (
    <Section title={t("Time in heart-rate zones")}>
      {!any ? (
        <p className="text-sm text-muted-foreground">
          {t("No heart-rate zones: set them in the user settings, and record a heart rate.")}
        </p>
      ) : (
        <>
          <div className="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-2 text-xs">
            {entries.map((entry) => (
              <div key={entry.id} className="contents">
                <span className="flex items-center gap-1.5 whitespace-nowrap text-muted-foreground">
                  <Swatch color={entry.color} />
                  {entry.label}
                </span>
                {entry.zones?.some((zone) => zone.seconds > 0) ? (
                  <ZoneStack zones={entry.zones} />
                ) : (
                  <span className="text-muted-foreground">{t("No heart rate")}</span>
                )}
              </div>
            ))}
          </div>
          <ul className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
            {ZONE_COLORS.map((color, i) => (
              <li key={color} className="flex items-center gap-1.5">
                <span
                  aria-hidden
                  className="size-2.5 rounded-sm"
                  style={{ backgroundColor: color }}
                />
                {t("Zone {{zone}}", { zone: i + 1 })}
              </li>
            ))}
          </ul>
        </>
      )}
    </Section>
  );
}
