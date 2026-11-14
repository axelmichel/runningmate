import { invoke } from "@tauri-apps/api/core";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import type { ActivityFilter } from "@/bindings/ActivityFilter";
import type { ActivityPage } from "@/bindings/ActivityPage";
import type { ActivityQuery } from "@/bindings/ActivityQuery";
import type { ActivitySummary } from "@/bindings/ActivitySummary";
import type { ActivityUpdate } from "@/bindings/ActivityUpdate";
import type { BestPerformance } from "@/bindings/BestPerformance";
import type { DayTotal } from "@/bindings/DayTotal";
import type { Equipment } from "@/bindings/Equipment";
import type { EquipmentKind } from "@/bindings/EquipmentKind";
import type { FolderSyncStatus } from "@/bindings/FolderSyncStatus";
import type { ImportSummary } from "@/bindings/ImportSummary";
import type { Media } from "@/bindings/Media";
import type { RouteOutline } from "@/bindings/RouteOutline";
import type { SystemSettings } from "@/bindings/SystemSettings";
import type { User } from "@/bindings/User";
import type { UserInput } from "@/bindings/UserInput";
import type { ViewMode } from "@/bindings/ViewMode";
import type { Weather } from "@/bindings/Weather";
import type { ZonesInput } from "@/bindings/ZonesInput";

export type AppInfo = {
  version: string;
  dataDir: string;
};

/** Typed wrappers for the Tauri commands in `src-tauri/src/commands`. */
export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),

  // Activities
  listActivities: (query: ActivityQuery) => invoke<ActivityPage>("list_activities", { query }),
  activitySummary: (viewMode: ViewMode, filter?: ActivityFilter) =>
    invoke<ActivitySummary>("activity_summary", { viewMode, filter }),
  activityHeatmap: (viewMode: ViewMode, filter?: ActivityFilter) =>
    invoke<DayTotal[]>("activity_heatmap", { viewMode, filter }),
  getActivity: (id: number) => invoke<ActivityDetail>("get_activity", { id }),
  updateActivity: (update: ActivityUpdate) => invoke<ActivityDetail>("update_activity", { update }),
  setActivityEquipment: (activityId: number, equipmentId: number | null) =>
    invoke<ActivityDetail>("set_activity_equipment", { activityId, equipmentId }),
  deleteActivity: (id: number) => invoke<void>("delete_activity", { id }),
  activityTypeDetails: (viewMode: ViewMode) =>
    invoke<string[]>("activity_type_details", { viewMode }),

  // Import and folder sync (progress arrives as `import://progress` events)
  importFiles: (paths: string[]) => invoke<ImportSummary>("import_files", { paths }),
  reimportActivity: (id: number) => invoke<void>("reimport_activity", { id }),
  activityRoute: (id: number) => invoke<RouteOutline | null>("activity_route", { id }),
  folderSyncStatus: () => invoke<FolderSyncStatus>("folder_sync_status"),
  setSyncFolder: (folder: string) => invoke<FolderSyncStatus>("set_sync_folder", { folder }),
  syncFolder: () => invoke<ImportSummary>("sync_folder"),

  // Media, weather, best performances
  listMedia: (activityId: number) => invoke<Media[]>("list_media", { activityId }),
  addMedia: (activityId: number, sourcePath: string) =>
    invoke<Media>("add_media", { activityId, sourcePath }),
  deleteMedia: (id: number) => invoke<void>("delete_media", { id }),
  getWeather: (activityId: number) => invoke<Weather | null>("get_weather", { activityId }),
  activityBestPerformances: (activityId: number) =>
    invoke<BestPerformance[]>("activity_best_performances", { activityId }),
  topBestPerformances: (viewMode: ViewMode) =>
    invoke<BestPerformance[]>("top_best_performances", { viewMode }),

  // User, heart-rate zones, shoes and bikes
  getUser: () => invoke<User | null>("get_user"),
  saveUser: (input: UserInput) => invoke<User>("save_user", { input }),
  saveHeartRateZones: (input: ZonesInput) => invoke<User>("save_heart_rate_zones", { input }),
  suggestedHeartRateZones: () =>
    invoke<[number, number, number, number, number]>("suggested_heart_rate_zones"),
  listEquipment: (kind: EquipmentKind) => invoke<Equipment[]>("list_equipment", { kind }),
  addEquipment: (kind: EquipmentKind, name: string, weight: number | null, inUse: boolean) =>
    invoke<Equipment[]>("add_equipment", { kind, name, weight, inUse }),
  setEquipmentInUse: (kind: EquipmentKind, id: number, inUse: boolean) =>
    invoke<Equipment[]>("set_equipment_in_use", { kind, id, inUse }),
  deleteEquipment: (kind: EquipmentKind, id: number) =>
    invoke<Equipment[]>("delete_equipment", { kind, id }),

  // System settings
  getSettings: () => invoke<SystemSettings>("get_settings"),
  saveSettings: (settings: SystemSettings) => invoke<SystemSettings>("save_settings", { settings }),
};
