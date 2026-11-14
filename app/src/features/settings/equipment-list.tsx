import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import type { Equipment } from "@/bindings/Equipment";
import type { EquipmentKind } from "@/bindings/EquipmentKind";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { t } from "@/i18n";
import { api } from "@/lib/api";
import { formatNumber } from "@/lib/format";

/** Shoes or bikes with their distance, an in-use switch, delete and an add form. */
export function EquipmentList({ kind }: { kind: EquipmentKind }) {
  const queryClient = useQueryClient();
  const bikes = kind === "bike";
  // Separate texts per kind, since German nouns differ in gender and plural.
  const text = bikes
    ? {
        none: t("No bikes yet."),
        add: t("New bike"),
        retired: t(
          "Retired bikes (not in use) keep their distance but aren't offered for new activities.",
        ),
        unassign: t(
          "Activities using it will no longer have a bike assigned. To keep the history, mark it as not in use instead.",
        ),
      }
    : {
        none: t("No shoes yet."),
        add: t("New shoe"),
        retired: t(
          "Retired shoes (not in use) keep their distance but aren't offered for new activities.",
        ),
        unassign: t(
          "Activities using them will no longer have shoes assigned. To keep the history, mark them as not in use instead.",
        ),
      };
  const [name, setName] = useState("");
  const [weight, setWeight] = useState("");
  const [deleting, setDeleting] = useState<Equipment | null>(null);
  const { data: items = [] } = useQuery({
    queryKey: ["equipment", kind],
    queryFn: () => api.listEquipment(kind),
  });
  // Every equipment command returns the updated list.
  const handlers = {
    onSuccess: (list: Equipment[]) => queryClient.setQueryData(["equipment", kind], list),
    onError: (error: unknown) => toast.error(String(error)),
  };
  const add = useMutation({
    mutationFn: () =>
      api.addEquipment(kind, name.trim(), bikes && weight ? Number(weight) : null, true),
    ...handlers,
  });
  const setInUse = useMutation({
    mutationFn: ({ id, inUse }: { id: number; inUse: boolean }) =>
      api.setEquipmentInUse(kind, id, inUse),
    ...handlers,
  });
  const remove = useMutation({
    mutationFn: (id: number) => api.deleteEquipment(kind, id),
    ...handlers,
  });
  const weightValue = Number(weight);
  const canAdd =
    name.trim() !== "" && (!bikes || weight === "" || (weightValue > 0 && weightValue < 50));

  return (
    <div className="grid max-w-2xl gap-4">
      <div className="overflow-hidden rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t("Name")}</TableHead>
              {bikes && <TableHead className="text-right">{t("Weight (kg)")}</TableHead>}
              <TableHead className="text-right">{t("Distance (km)")}</TableHead>
              <TableHead className="w-20 text-center">{t("In use")}</TableHead>
              <TableHead className="w-10" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {items.map((item) => (
              <TableRow key={item.id}>
                <TableCell className="font-medium">{item.name}</TableCell>
                {bikes && (
                  <TableCell className="text-right tabular-nums">
                    {formatNumber(item.weight, 1)}
                  </TableCell>
                )}
                <TableCell className="text-right tabular-nums">
                  {formatNumber(item.distance, 2)}
                </TableCell>
                <TableCell className="text-center">
                  <Checkbox
                    aria-label={t("{{name}} in use", { name: item.name })}
                    checked={item.inUse}
                    onCheckedChange={(checked) =>
                      setInUse.mutate({ id: item.id, inUse: checked === true })
                    }
                  />
                </TableCell>
                <TableCell>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={t("Delete {{name}}", { name: item.name })}
                    className="text-muted-foreground hover:text-destructive"
                    onClick={() => setDeleting(item)}
                  >
                    <Trash2 />
                  </Button>
                </TableCell>
              </TableRow>
            ))}
            {items.length === 0 && (
              <TableRow>
                <TableCell
                  colSpan={bikes ? 5 : 4}
                  className="py-6 text-center text-muted-foreground"
                >
                  {text.none}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      <form
        className="flex flex-wrap items-end gap-3"
        onSubmit={(event) => {
          event.preventDefault();
          if (!canAdd) return;
          add.mutate(undefined, {
            onSuccess: () => {
              setName("");
              setWeight("");
            },
          });
        }}
      >
        <div className="grid min-w-56 flex-1 gap-1.5">
          <Label htmlFor={`${kind}-name`}>{text.add}</Label>
          <Input
            id={`${kind}-name`}
            value={name}
            placeholder={t("Name")}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
        {bikes && (
          <div className="grid w-32 gap-1.5">
            <Label htmlFor="bike-weight">{t("Weight (kg)")}</Label>
            <Input
              id="bike-weight"
              type="number"
              step="0.1"
              value={weight}
              onChange={(e) => setWeight(e.target.value)}
            />
          </div>
        )}
        <Button type="submit" disabled={!canAdd || add.isPending}>
          <Plus />
          {t("Add")}
        </Button>
      </form>
      <p className="text-xs text-muted-foreground">{text.retired}</p>

      <AlertDialog open={deleting !== null} onOpenChange={(open) => !open && setDeleting(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("Delete “{{name}}”?", { name: deleting?.name })}</AlertDialogTitle>
            <AlertDialogDescription>{text.unassign}</AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("Cancel")}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => deleting && remove.mutate(deleting.id)}
            >
              {t("Delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
