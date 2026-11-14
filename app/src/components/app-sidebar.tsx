import { Gauge, HeartPulse, RefreshCw, Search, Settings, User } from "lucide-react";

import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar";
import { VIEW_MODES, type ViewMode } from "@/features/activities/view-mode";
import { t } from "@/i18n";

type AppSidebarProps = {
  page: "dashboard" | "activities" | "health";
  /** The activity view, marked while the activities page is shown. */
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  onPageChange: (page: "dashboard" | "health") => void;
  searchOpen: boolean;
  onSearchToggle: () => void;
  onSync: () => void;
  onUserSettings: () => void;
  onSettings: () => void;
};

export function AppSidebar({
  page,
  viewMode,
  onViewModeChange,
  onPageChange,
  searchOpen,
  onSearchToggle,
  onSync,
  onUserSettings,
  onSettings,
}: AppSidebarProps) {
  return (
    <Sidebar
      collapsible="icon"
      className="top-(--titlebar-height) h-[calc(100svh-var(--titlebar-height))]"
    >
      <SidebarHeader>
        <div className="px-2 py-1 text-lg font-semibold group-data-[collapsible=icon]:hidden">
          RunningMate
        </div>
      </SidebarHeader>
      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={page === "dashboard"}
                  tooltip={`${t("Dashboard")} (⇧⌘D)`}
                  onClick={() => onPageChange("dashboard")}
                >
                  <Gauge />
                  <span>{t("Dashboard")}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              {VIEW_MODES.map(({ mode, label, icon: Icon }) => (
                <SidebarMenuItem key={mode}>
                  <SidebarMenuButton
                    isActive={page === "activities" && viewMode === mode}
                    tooltip={t(label)}
                    onClick={() => onViewModeChange(mode)}
                  >
                    <Icon />
                    <span>{t(label)}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
        <SidebarGroup>
          <SidebarGroupContent>
            <SidebarMenu>
              <SidebarMenuItem>
                <SidebarMenuButton
                  isActive={page === "health"}
                  tooltip={`${t("Health data")} (⇧⌘H)`}
                  onClick={() => onPageChange("health")}
                >
                  <HeartPulse />
                  <span>{t("Health data")}</span>
                </SidebarMenuButton>
              </SidebarMenuItem>
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip={`${t("Sync")} (⌘O)`} onClick={onSync}>
              <RefreshCw />
              <span>{t("Sync")}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton
              tooltip={`${t("Search")} (⌘F)`}
              isActive={searchOpen}
              onClick={onSearchToggle}
            >
              <Search />
              <span>{t("Search")}</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
          {[
            { label: t("User"), tooltip: t("User settings"), icon: User, onClick: onUserSettings },
            {
              label: t("Settings"),
              tooltip: `${t("Settings")} (⌘,)`,
              icon: Settings,
              onClick: onSettings,
            },
          ].map(({ label, tooltip, icon: Icon, onClick }) => (
            <SidebarMenuItem key={label}>
              <SidebarMenuButton tooltip={tooltip} onClick={onClick}>
                <Icon />
                <span>{label}</span>
              </SidebarMenuButton>
            </SidebarMenuItem>
          ))}
        </SidebarMenu>
      </SidebarFooter>
    </Sidebar>
  );
}
