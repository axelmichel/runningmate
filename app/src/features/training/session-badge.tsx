import { CalendarCheck } from "lucide-react";

import { t } from "@/i18n";
import { cn } from "@/lib/utils";

import { KIND_LABELS, VERDICT_COLORS, checkText } from "./training-math";
import { useActivitySession } from "./use-training";

/** Marker for an activity that did a training session, with how it went. */
export function SessionBadge({
  activityId,
  enabled = true,
  className,
}: {
  activityId: number;
  enabled?: boolean;
  className?: string;
}) {
  const { data: found } = useActivitySession(activityId, enabled);
  if (!found) return null;
  const { status } = found;
  const label = `${found.planName}: ${t(KIND_LABELS[status.session.kind])} · ${checkText(status)}`;
  return (
    <span
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex shrink-0 items-center rounded-full bg-muted px-2 py-0.5",
        VERDICT_COLORS[status.check.verdict],
        className,
      )}
    >
      <CalendarCheck className="size-3.5" />
    </span>
  );
}
