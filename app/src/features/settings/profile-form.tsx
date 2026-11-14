import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { toast } from "sonner";

import type { User } from "@/bindings/User";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";

import { validateProfile, type ProfileForm } from "./validation";

const FIELDS: { key: keyof ProfileForm; label: string; placeholder?: string; type?: string }[] = [
  { key: "name", label: tk("Name") },
  { key: "birthday", label: tk("Birthday"), placeholder: tk("DD.MM.YYYY") },
  { key: "weight", label: tk("Weight (kg)"), type: "number" },
  { key: "height", label: tk("Height (cm)"), type: "number" },
  { key: "hrMin", label: tk("Resting heart rate (bpm)"), type: "number" },
  { key: "hrMax", label: tk("Maximum heart rate (bpm)"), type: "number" },
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
      }),
    onSuccess: (saved) => {
      queryClient.setQueryData(["user"], saved);
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
