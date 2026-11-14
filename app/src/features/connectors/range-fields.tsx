import { X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { t } from "@/i18n";

import { OPEN_RANGE, type RangeInput } from "./range";

/** Optional From/To dates limiting a sync. */
export function RangeFields({
  id,
  value,
  onChange,
  disabled,
  hint,
}: {
  id: string;
  value: RangeInput;
  onChange: (value: RangeInput) => void;
  disabled?: boolean;
  hint?: string;
}) {
  const set = (key: keyof RangeInput) => (event: { target: { value: string } }) =>
    onChange({ ...value, [key]: event.target.value });
  return (
    <div className="grid gap-1.5">
      <div className="flex flex-wrap items-end gap-3">
        <div className="grid gap-1.5">
          <Label htmlFor={`${id}-from`}>{t("From")}</Label>
          <Input
            id={`${id}-from`}
            type="date"
            className="w-40"
            value={value.from}
            max={value.to || undefined}
            disabled={disabled}
            onChange={set("from")}
          />
        </div>
        <div className="grid gap-1.5">
          <Label htmlFor={`${id}-to`}>{t("To")}</Label>
          <Input
            id={`${id}-to`}
            type="date"
            className="w-40"
            value={value.to}
            min={value.from || undefined}
            disabled={disabled}
            onChange={set("to")}
          />
        </div>
        {(value.from || value.to) && (
          <Button
            variant="ghost"
            size="sm"
            disabled={disabled}
            onClick={() => onChange(OPEN_RANGE)}
          >
            <X />
            {t("Any time")}
          </Button>
        )}
      </div>
      <p className="text-xs text-muted-foreground">
        {hint ?? t("Optional. Newest activities are imported first.")}
      </p>
    </div>
  );
}
