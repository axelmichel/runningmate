import { FolderSync, Import, Search, Settings, User } from "lucide-react";

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

type AppSidebarProps = {
  viewMode: ViewMode;
  onViewModeChange: (mode: ViewMode) => void;
  searchOpen: boolean;
  onSearchToggle: () => void;
  onImport: () => void;
  onFolderSync: () => void;
  onUserSettings: () => void;
  onSettings: () => void;
};

export function AppSidebar({
  viewMode,
  onViewModeChange,
  searchOpen,
  onSearchToggle,
  onImport,
  onFolderSync,
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
              {VIEW_MODES.map(({ mode, label, icon: Icon }) => (
                <SidebarMenuItem key={mode}>
                  <SidebarMenuButton
                    isActive={viewMode === mode}
                    tooltip={label}
                    onClick={() => onViewModeChange(mode)}
                  >
                    <Icon />
                    <span>{label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Import TCX (⌘O)" onClick={onImport}>
              <Import />
              <span>Import</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Sync folder" onClick={onFolderSync}>
              <FolderSync />
              <span>Sync folder</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
          <SidebarMenuItem>
            <SidebarMenuButton tooltip="Search (⌘F)" isActive={searchOpen} onClick={onSearchToggle}>
              <Search />
              <span>Search</span>
            </SidebarMenuButton>
          </SidebarMenuItem>
          {[
            { label: "User", tooltip: "User settings", icon: User, onClick: onUserSettings },
            { label: "Settings", tooltip: "Settings (⌘,)", icon: Settings, onClick: onSettings },
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
