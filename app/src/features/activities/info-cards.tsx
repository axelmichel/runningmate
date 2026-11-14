import type { ActivitySummary } from "@/bindings/ActivitySummary";
import { Card, CardContent } from "@/components/ui/card";
import { t } from "@/i18n";
import { formatHours, formatNumber } from "@/lib/format";

export function InfoCards({ summary }: { summary: ActivitySummary | undefined }) {
  const items = [
    { label: t("Activities"), value: formatNumber(summary?.count) },
    { label: t("Distance"), value: `${formatNumber(summary?.distance, 2)} km` },
    { label: t("Duration"), value: summary ? formatHours(summary.duration) : "–" },
    { label: t("Elevation"), value: `${formatNumber(summary?.elevationGain)} m` },
  ];
  return (
    <div className="grid grid-cols-2 gap-3 xl:grid-cols-4">
      {items.map((item) => (
        <Card key={item.label} size="sm" className="gap-1">
          <CardContent>
            <div className="text-xs text-muted-foreground">{item.label}</div>
            <div className="text-xl font-semibold text-primary tabular-nums">{item.value}</div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
