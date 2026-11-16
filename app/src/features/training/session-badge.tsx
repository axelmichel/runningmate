import { CalendarCheck } from "lucide-react";

import { t } from "@/i18n";
import { cn } from "@/lib/utils";

import { KIND_LABELS, VERDICT_COLORS, checkText } from "./training-math";
import { useActivitySession } from "./use-training";

/**
 * Marker for an activity that did a training session, with how it went; `labelled` adds the
 * kind of session.
 */
export function SessionBadge({
  activityId,
  enabled = true,
  labelled = false,
  className,
}: {
  activityId: number;
  enabled?: boolean;
  labelled?: boolean;
  className?: string;
}) {
  const { data: found } = useActivitySession(activityId, enabled);
  if (!found) return null;
  const { status } = found;
  const kind = t(KIND_LABELS[status.session.kind]);
  const label = `${found.planName}: ${kind} · ${checkText(status)}`;
  return (
    <span
      title={label}
      aria-label={label}
      className={cn(
        "inline-flex shrink-0 items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-semibold",
        VERDICT_COLORS[status.check.verdict],
        className,
      )}
    >
      <CalendarCheck className="size-3.5" />
      {labelled && kind}
    </span>
  );
}
