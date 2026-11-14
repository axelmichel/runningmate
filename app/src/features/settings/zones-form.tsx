import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import type { User } from "@/bindings/User";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { t } from "@/i18n";
import { api } from "@/lib/api";

import { validateZones } from "./validation";

const ZONE_COLORS = ["#35193e", "#701f57", "#ad1759", "#f37651", "#f6b48f"];

/** VO2max and zone upper bounds (the "Heart Rate Zones" page of the Python settings). */
export function ZonesFormView({ user }: { user: User }) {
  const queryClient = useQueryClient();
  const suggested = useQuery({
    queryKey: ["suggestedZones", user.hrMax, user.age, user.birthday],
    queryFn: api.suggestedHeartRateZones,
    enabled: user.zones == null,
  });
  const initial = user.zones ?? suggested.data;
  const [zones, setZones] = useState<string[] | null>(null);
  const [vo2max, setVo2max] = useState(user.vo2max?.toString() ?? "");
  const values = zones ?? initial?.map(String) ?? ["", "", "", "", ""];
  const error = validateZones(values);
  const vo2 = vo2max.trim() === "" ? 0 : Number(vo2max);
  const vo2Invalid = !(vo2 >= 0 && vo2 <= 100);

  const save = useMutation({
    mutationFn: () =>
      api.saveHeartRateZones({
        vo2max: vo2,
        hrMin: user.hrMin ?? 0,
        zones: values.map(Number) as [number, number, number, number, number],
      }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["user"], saved);
      queryClient.invalidateQueries({ queryKey: ["zones"] });
      queryClient.invalidateQueries({ queryKey: ["heartRateTrend"] });
      toast.success(t("Heart-rate zones saved"));
    },
    onError: (e) => toast.error(t("Could not save: {{error}}", { error: String(e) })),
  });

  return (
    <form
      className="grid max-w-md gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (!error && !vo2Invalid) save.mutate();
      }}
    >
      {user.zones == null && (
        <p className="text-sm text-muted-foreground">
          {t("Suggested from your maximum heart rate and age (60–100 %). Adjust and save them.")}
        </p>
      )}
      {values.map((value, i) => {
        const lower = i === 0 ? Math.floor(Number(values[4]) / 2) : Number(values[i - 1]) + 1;
        return (
          <div key={i} className="grid grid-cols-[1rem_6rem_1fr_6rem] items-center gap-3">
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
      <div className="grid grid-cols-[1rem_6rem_1fr_6rem] items-center gap-3">
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
        <Button type="submit" disabled={Boolean(error) || vo2Invalid || save.isPending}>
          {t("Save")}
        </Button>
      </div>
    </form>
  );
}
