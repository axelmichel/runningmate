import { useQuery } from "@tanstack/react-query";
import { Bike, Footprints, HeartPulse, UserRound } from "lucide-react";
import { useState } from "react";

import { NavTabs, type NavItem } from "@/components/nav-tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsContent } from "@/components/ui/tabs";
import { api } from "@/lib/api";

import { EquipmentList } from "./equipment-list";
import { ProfileFormView } from "./profile-form";
import { ZonesFormView } from "./zones-form";

type Page = "profile" | "zones" | "shoes" | "bikes";

/** User profile, heart-rate zones, shoes and bikes (`ui/window_user_settings.py`). */
export function UserSettingsDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [page, setPage] = useState<Page>("profile");
  const { data: user, isPending } = useQuery({
    queryKey: ["user"],
    queryFn: api.getUser,
    enabled: open,
  });
  // Zones, shoes and bikes need a saved profile, as in Python.
  const items: (NavItem & { value: Page })[] = [
    { value: "profile", label: "Profile", icon: UserRound },
    { value: "zones", label: "Heart rate zones", icon: HeartPulse, disabled: !user },
    { value: "shoes", label: "Shoes", icon: Footprints, disabled: !user },
    { value: "bikes", label: "Bikes", icon: Bike, disabled: !user },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex h-[70vh] max-w-4xl flex-col gap-4 sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>User settings</DialogTitle>
          <DialogDescription className="sr-only">
            Profile, heart-rate zones, shoes and bikes
          </DialogDescription>
        </DialogHeader>
        {isPending ? (
          <Skeleton className="size-full" />
        ) : (
          <Tabs
            value={user ? page : "profile"}
            onValueChange={(value) => setPage(value as Page)}
            orientation="vertical"
            className="min-h-0 flex-1 flex-row gap-6"
          >
            <NavTabs items={items} />
            <div className="scrollbar-visible min-h-0 min-w-0 flex-1 overflow-auto pr-1">
              <TabsContent value="profile">
                <ProfileFormView key={user?.id ?? "new"} user={user ?? null} />
              </TabsContent>
              {user && (
                <>
                  <TabsContent value="zones">
                    <ZonesFormView key={JSON.stringify(user.zones)} user={user} />
                  </TabsContent>
                  <TabsContent value="shoes">
                    <EquipmentList kind="shoe" />
                  </TabsContent>
                  <TabsContent value="bikes">
                    <EquipmentList kind="bike" />
                  </TabsContent>
                </>
              )}
            </div>
          </Tabs>
        )}
      </DialogContent>
    </Dialog>
  );
}
