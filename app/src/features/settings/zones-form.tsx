import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import type { User } from "@/bindings/User";
import type { ZoneMethod } from "@/bindings/ZoneMethod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";

import { validateZones } from "./validation";
import { customFloor, invalidateZones, zoneRanges } from "./zones";

const ZONE_COLORS = ["#35193e", "#701f57", "#ad1759", "#f37651", "#f6b48f"];

const METHODS: [ZoneMethod, string, string][] = [
  ["custom", tk("Custom"), tk("Enter the upper bound of each zone yourself.")],
  [
    "percentMax",
    tk("Percent of maximum heart rate"),
    tk("60, 70, 80, 90 and 100 % of your maximum heart rate; zone 1 starts at 50 %."),
  ],
  [
    "karvonen",
    tk("Karvonen (heart-rate reserve)"),
    tk(
      "The same shares of the range between resting and maximum heart rate, added to the resting heart rate.",
    ),
  ],
];

const ROW = "grid grid-cols-[1rem_6rem_1fr_6rem] items-center gap-3";

/** Zone method, zone upper bounds and VO2max (the "Heart Rate Zones" page of the Python settings). */
export function ZonesFormView({ user }: { user: User }) {
  const queryClient = useQueryClient();
  const [method, setMethod] = useState<ZoneMethod>(user.zoneMethod);
  const custom = method === "custom";
  const suggested = useQuery({
    queryKey: ["suggestedZones", user.hrMax, user.age, user.birthday],
    queryFn: api.suggestedHeartRateZones,
    enabled: custom && user.customZones == null,
  });
  // The derived zones come from the backend, so the form shows what the analyses will use.
  const preview = useQuery({
    queryKey: ["zonePreview", method, user.hrMin, user.hrMax],
    queryFn: () => api.previewHeartRateZones(method),
    enabled: !custom,
  });
  const initial = user.customZones ?? suggested.data;
  const [zones, setZones] = useState<string[] | null>(null);
  const [vo2max, setVo2max] = useState(user.vo2max?.toString() ?? "");
  const values = zones ?? initial?.map(String) ?? ["", "", "", "", ""];
  const error = custom ? validateZones(values) : null;
  const unavailable = !custom && preview.isSuccess && preview.data == null;
  const vo2 = vo2max.trim() === "" ? 0 : Number(vo2max);
  const vo2Invalid = !(vo2 >= 0 && vo2 <= 100);
  const blocked = Boolean(error) || (!custom && !preview.data) || vo2Invalid;

  const save = useMutation({
    mutationFn: () =>
      api.saveHeartRateZones({
        vo2max: vo2,
        hrMin: user.hrMin ?? 0,
        method,
        zones: custom
          ? (values.map(Number) as [number, number, number, number, number])
          : undefined,
      }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["user"], saved);
      invalidateZones(queryClient);
      toast.success(t("Heart-rate zones saved"));
    },
    onError: (e) => toast.error(t("Could not save: {{error}}", { error: String(e) })),
  });

  const customLower = customFloor(values.map(Number));

  return (
    <form
      className="grid max-w-md gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (!blocked) save.mutate();
      }}
    >
      <div className="grid gap-2">
        <Label>{t("Method")}</Label>
        <RadioGroup value={method} onValueChange={(value) => setMethod(value as ZoneMethod)}>
          {METHODS.map(([value, label, hint]) => (
            <div key={value} className="flex items-start gap-2">
              <RadioGroupItem value={value} id={`zone-method-${value}`} className="mt-0.5" />
              <div className="grid gap-0.5">
                <Label htmlFor={`zone-method-${value}`} className="font-normal">
                  {t(label)}
                </Label>
                <p className="text-xs text-muted-foreground">{t(hint)}</p>
              </div>
            </div>
          ))}
        </RadioGroup>
      </div>
      {custom ? (
        <>
          {user.customZones == null && (
            <p className="text-sm text-muted-foreground">
              {t(
                "Suggested from your maximum heart rate and age (60–100 %). Adjust and save them.",
              )}
            </p>
          )}
          {values.map((value, i) => {
            const lower = i === 0 ? customLower : Number(values[i - 1]) + 1;
            return (
              <div key={i} className={ROW}>
                <span className="size-3 rounded-full" style={{ backgroundColor: ZONE_COLORS[i] }} />
                <Label htmlFor={`zone-${i}`}>{t("Zone {{zone}}", { zone: i + 1 })}</Label>
                <span className="text-sm text-muted-foreground tabular-nums">
                  {Number.isFinite(lower) ? `${lower} –` : ""}
                </span>
                <Input
                  id={`zone-${i}`}
                  type="number"
                  value={value}
                  onChange={(event) => {
                    const next = [...values];
                    next[i] = event.target.value;
                    setZones(next);
                  }}
                />
              </div>
            );
          })}
          {error && <p className="text-xs text-destructive">{t(error)}</p>}
        </>
      ) : unavailable ? (
        <p className="text-sm text-muted-foreground">
          {method === "karvonen"
            ? t("Enter your resting and maximum heart rate in the profile to use this method.")
            : t("Enter your maximum heart rate in the profile to use this method.")}
        </p>
      ) : (
        preview.data &&
        zoneRanges(preview.data.floor, preview.data.upper).map(([lower, upper], i) => (
          <div key={i} className={ROW}>
            <span className="size-3 rounded-full" style={{ backgroundColor: ZONE_COLORS[i] }} />
            <span className="text-sm font-medium">{t("Zone {{zone}}", { zone: i + 1 })}</span>
            <span className="text-sm text-muted-foreground tabular-nums">{`${lower} –`}</span>
            <span className="text-sm tabular-nums">{upper}</span>
          </div>
        ))
      )}
      {!custom && !unavailable && (
        <p className="text-xs text-muted-foreground">
          {t("Calculated from the profile; they follow changes to your heart rates there.")}
        </p>
      )}
      <div className={ROW}>
        <span />
        <Label htmlFor="vo2max">VO2max</Label>
        <span />
        <Input
          id="vo2max"
          type="number"
          step="0.1"
          value={vo2max}
          aria-invalid={vo2Invalid}
          onChange={(e) => setVo2max(e.target.value)}
        />
      </div>
      <div>
        <Button type="submit" disabled={blocked || save.isPending}>
          {t("Save")}
        </Button>
      </div>
    </form>
  );
}
