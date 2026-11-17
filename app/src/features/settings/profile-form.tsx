import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import type { Gender } from "@/bindings/Gender";
import type { User } from "@/bindings/User";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";

import { validateProfile, type ProfileForm } from "./validation";
import { invalidateZones } from "./zones";

const FIELDS: { key: keyof ProfileForm; label: string; placeholder?: string; type?: string }[] = [
  { key: "name", label: tk("Name") },
  { key: "birthday", label: tk("Birthday"), placeholder: tk("DD.MM.YYYY") },
  { key: "weight", label: tk("Weight (kg)"), type: "number" },
  { key: "height", label: tk("Height (cm)"), type: "number" },
  { key: "hrMin", label: tk("Resting heart rate (bpm)"), type: "number" },
  { key: "hrMax", label: tk("Maximum heart rate (bpm)"), type: "number" },
];

const GENDERS: [Gender, string][] = [
  ["female", tk("Female")],
  ["male", tk("Male")],
];

/** General user data (the "General" page of `window_user_settings.py`). */
export function ProfileFormView({ user }: { user: User | null }) {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<ProfileForm>({
    name: user?.name ?? "",
    weight: user?.weight?.toString() ?? "",
    height: user?.height?.toString() ?? "",
    birthday: user?.birthday ?? "",
    hrMin: user?.hrMin?.toString() ?? "",
    hrMax: user?.hrMax?.toString() ?? "",
  });
  // Optional; only used to compare VO2max with people of the same age and sex.
  const [gender, setGender] = useState<Gender | null>(user?.gender ?? null);
  const [touched, setTouched] = useState(false);
  const errors = validateProfile(form);
  const valid = Object.keys(errors).length === 0;

  const save = useMutation({
    mutationFn: () =>
      api.saveUser({
        name: form.name.trim(),
        weight: Number(form.weight),
        height: Number(form.height),
        birthday: form.birthday.trim(),
        hrMin: Number(form.hrMin),
        hrMax: Number(form.hrMax),
        gender: gender ?? undefined,
      }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["user"], saved);
      // Zones from % of max or Karvonen follow the heart rates of the profile.
      invalidateZones(queryClient);
      toast.success(t("Profile saved"));
    },
    onError: (error) => toast.error(t("Could not save: {{error}}", { error: String(error) })),
  });

  return (
    <form
      className="grid max-w-xl gap-4"
      onSubmit={(event) => {
        event.preventDefault();
        setTouched(true);
        if (valid) save.mutate();
      }}
    >
      <div className="grid gap-4 sm:grid-cols-2">
        {FIELDS.map(({ key, label, placeholder, type }) => (
          <div key={key} className="grid gap-1.5">
            <Label htmlFor={`profile-${key}`}>{t(label)}</Label>
            <Input
              id={`profile-${key}`}
              type={type}
              placeholder={placeholder && t(placeholder)}
              value={form[key]}
              aria-invalid={touched && Boolean(errors[key])}
              onChange={(event) => setForm({ ...form, [key]: event.target.value })}
            />
            {touched && errors[key] && <p className="text-xs text-destructive">{t(errors[key])}</p>}
          </div>
        ))}
        <div className="grid gap-1.5">
          <Label htmlFor="profile-gender">{t("Sex")}</Label>
          <Select value={gender ?? ""} onValueChange={(value) => setGender(value as Gender)}>
            <SelectTrigger id="profile-gender" className="w-full">
              <SelectValue placeholder={t("Not specified")} />
            </SelectTrigger>
            <SelectContent>
              {GENDERS.map(([value, label]) => (
                <SelectItem key={value} value={value}>
                  {t(label)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            {t("Optional. Compares your cardio fitness with people of your age and sex.")}
          </p>
        </div>
      </div>
      <p className="text-xs text-muted-foreground">
        {t(
          "Your age is calculated from the birthday. Heart-rate zones, shoes and bikes need a saved profile.",
        )}
      </p>
      <div>
        <Button type="submit" disabled={save.isPending}>
          {t("Save")}
        </Button>
      </div>
    </form>
  );
}
