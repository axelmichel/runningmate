import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { invalidateGoals } from "@/features/goals/goal-math";
import { invalidateTraining } from "@/features/training/training-math";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber, parseDuration } from "@/lib/format";

import { MediaGallery } from "./media-gallery";

/** Unix timestamp → `datetime-local` input value in local time. */
function toLocalInput(timestamp: number | null): string {
  if (timestamp == null) return "";
  const d = new Date(timestamp * 1000);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function durationText(seconds: number | null): string {
  if (seconds == null) return "";
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${Math.floor(seconds / 3600)}:${pad(Math.floor((seconds % 3600) / 60))}:${pad(seconds % 60)}`;
}

function EquipmentPicker({ activity }: { activity: ActivityDetail }) {
  const queryClient = useQueryClient();
  const kind = activity.viewMode === "Cycling" ? "bike" : "shoe";
  const { data: equipment = [] } = useQuery({
    queryKey: ["equipment", kind],
    queryFn: () => api.listEquipment(kind),
  });
  const assign = useMutation({
    mutationFn: (id: number | null) => api.setActivityEquipment(activity.id, id),
    onSuccess: (updated) => {
      queryClient.setQueryData(["activity", activity.id], updated);
      queryClient.invalidateQueries({ queryKey: ["equipment", kind] });
    },
    onError: (error) =>
      toast.error(
        kind === "bike"
          ? t("Could not change bike: {{error}}", { error: String(error) })
          : t("Could not change shoes: {{error}}", { error: String(error) }),
      ),
  });
  // Retired items stay listed only while they're assigned to this activity.
  const options = equipment.filter((e) => e.inUse || e.id === activity.equipment?.id);
  return (
    <div className="grid content-start gap-2 self-start">
      <Label>{kind === "bike" ? t("Bike") : t("Shoes")}</Label>
      <RadioGroup
        value={String(activity.equipment?.id ?? "none")}
        onValueChange={(value) => assign.mutate(value === "none" ? null : Number(value))}
        disabled={assign.isPending}
      >
        {[
          { id: "none", label: t("None") },
          ...options.map((e) => ({
            id: String(e.id),
            label: `${e.name} · ${formatNumber(e.distance)} km`,
          })),
        ].map((option) => (
          <div key={option.id} className="flex items-center gap-2">
            <RadioGroupItem value={option.id} id={`equipment-${option.id}`} />
            <Label htmlFor={`equipment-${option.id}`} className="font-normal">
              {option.label}
            </Label>
          </div>
        ))}
      </RadioGroup>
      <p className="text-xs text-muted-foreground">
        {t("Changes apply right away and move the distance between items.")}
      </p>
    </div>
  );
}

/** Edit page. Unlike the Python app, every field shown here is saved. */
export function EditTab({ activity, onSaved }: { activity: ActivityDetail; onSaved: () => void }) {
  const queryClient = useQueryClient();
  const { data: typeDetails = [] } = useQuery({
    queryKey: ["typeDetails", activity.viewMode],
    queryFn: () => api.activityTypeDetails(activity.viewMode),
  });
  const [form, setForm] = useState({
    title: activity.title ?? "",
    comment: activity.comment ?? "",
    typeDetail: activity.typeDetail ?? "",
    date: toLocalInput(activity.date),
    duration: durationText(activity.duration),
    distance: activity.distance?.toString() ?? "",
    elevation: activity.elevationGain?.toString() ?? "",
  });
  const duration = parseDuration(form.duration);
  const distance = Number(form.distance);
  const elevation = Number(form.elevation);
  const date = form.date ? Math.floor(new Date(form.date).getTime() / 1000) : null;
  const valid =
    form.title.trim() !== "" && duration != null && date != null && distance >= 0 && elevation >= 0;

  const save = useMutation({
    mutationFn: () =>
      api.updateActivity({
        id: activity.id,
        title: form.title.trim(),
        comment: form.comment,
        typeDetail: form.typeDetail || null,
        date: date!,
        duration: duration!,
        distance,
        elevationGain: elevation,
      }),
    onSuccess: (updated) => {
      queryClient.setQueryData(["activity", activity.id], updated);
      queryClient.invalidateQueries({ queryKey: ["activities"] });
      invalidateGoals(queryClient);
      invalidateTraining(queryClient);
      toast.success(t("Activity saved"));
      onSaved();
    },
    onError: (error) => toast.error(t("Could not save: {{error}}", { error: String(error) })),
  });

  const set = (key: keyof typeof form) => (event: { target: { value: string } }) =>
    setForm({ ...form, [key]: event.target.value });

  return (
    <div className="grid gap-6 lg:grid-cols-[1fr_16rem]">
      <div className="grid content-start gap-8">
        <form
          className="grid content-start gap-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (valid) save.mutate();
          }}
        >
          <div className="grid gap-2">
            <Label htmlFor="title">{t("Title")}</Label>
            <Input id="title" value={form.title} onChange={set("title")} />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="comment">{t("Comment")}</Label>
            <Textarea id="comment" rows={5} value={form.comment} onChange={set("comment")} />
          </div>
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="grid gap-2">
              <Label>{t("Type")}</Label>
              <Select
                value={form.typeDetail || "default"}
                onValueChange={(v) => setForm({ ...form, typeDetail: v === "default" ? "" : v })}
              >
                <SelectTrigger className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="default">{t(activity.activityType ?? "Default")}</SelectItem>
                  {typeDetails.map((detail) => (
                    <SelectItem key={detail} value={detail}>
                      {t(detail)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="grid gap-2">
              <Label htmlFor="date">{t("Start")}</Label>
              <Input id="date" type="datetime-local" value={form.date} onChange={set("date")} />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="duration">{t("Duration (h:mm:ss)")}</Label>
              <Input
                id="duration"
                value={form.duration}
                onChange={set("duration")}
                aria-invalid={duration == null}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="distance">{t("Distance (km)")}</Label>
              <Input
                id="distance"
                type="number"
                min={0}
                step="0.01"
                value={form.distance}
                onChange={set("distance")}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="elevation">{t("Elevation (m)")}</Label>
              <Input
                id="elevation"
                type="number"
                min={0}
                step="1"
                value={form.elevation}
                onChange={set("elevation")}
              />
            </div>
          </div>
          <div>
            <Button type="submit" disabled={!valid || save.isPending}>
              {t("Save")}
            </Button>
          </div>
        </form>
        {/* Outside the form: its buttons would otherwise submit it. Media changes apply right
          away, like on the General page. */}
        <MediaGallery activityId={activity.id} />
      </div>
      {activity.viewMode !== "All" && <EquipmentPicker activity={activity} />}
    </div>
  );
}
