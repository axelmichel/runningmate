import { invoke } from "@tauri-apps/api/core";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import type { ActivityFilter } from "@/bindings/ActivityFilter";
import type { ActivityPage } from "@/bindings/ActivityPage";
import type { ActivityPhoto } from "@/bindings/ActivityPhoto";
import type { ActivityQuery } from "@/bindings/ActivityQuery";
import type { ActivitySegmentEffort } from "@/bindings/ActivitySegmentEffort";
import type { ActivitySummary } from "@/bindings/ActivitySummary";
import type { ActivityUpdate } from "@/bindings/ActivityUpdate";
import type { AiSummary } from "@/bindings/AiSummary";
import type { Baseline } from "@/bindings/Baseline";
import type { BestPerformance } from "@/bindings/BestPerformance";
import type { ConnectorLink } from "@/bindings/ConnectorLink";
import type { DateRange } from "@/bindings/DateRange";
import type { DayTotal } from "@/bindings/DayTotal";
import type { Equipment } from "@/bindings/Equipment";
import type { EquipmentKind } from "@/bindings/EquipmentKind";
import type { ExportCandidate } from "@/bindings/ExportCandidate";
import type { FitnessDay } from "@/bindings/FitnessDay";
import type { FixEdit } from "@/bindings/FixEdit";
import type { FixSource } from "@/bindings/FixSource";
import type { FolderSyncStatus } from "@/bindings/FolderSyncStatus";
import type { GarminSignInResult } from "@/bindings/GarminSignInResult";
import type { GarminStatus } from "@/bindings/GarminStatus";
import type { Goal } from "@/bindings/Goal";
import type { GoalHit } from "@/bindings/GoalHit";
import type { GoalInput } from "@/bindings/GoalInput";
import type { GoalProgress } from "@/bindings/GoalProgress";
import type { HealthReport } from "@/bindings/HealthReport";
import type { HeartRateTrend } from "@/bindings/HeartRateTrend";
import type { ImportOutcome } from "@/bindings/ImportOutcome";
import type { ImportSummary } from "@/bindings/ImportSummary";
import type { LibraryAsset } from "@/bindings/LibraryAsset";
import type { MatchedRun } from "@/bindings/MatchedRun";
import type { Media } from "@/bindings/Media";
import type { PendingRoute } from "@/bindings/PendingRoute";
import type { PhotosAccess } from "@/bindings/PhotosAccess";
import type { PlanDetail } from "@/bindings/PlanDetail";
import type { PlanInput } from "@/bindings/PlanInput";
import type { PlanSession } from "@/bindings/PlanSession";
import type { PlanSummary } from "@/bindings/PlanSummary";
import type { Preview } from "@/bindings/Preview";
import type { RouteHeatmap } from "@/bindings/RouteHeatmap";
import type { RouteOutline } from "@/bindings/RouteOutline";
import type { RouteSegment } from "@/bindings/RouteSegment";
import type { SegmentDetail } from "@/bindings/SegmentDetail";
import type { SegmentRow } from "@/bindings/SegmentRow";
import type { SessionInput } from "@/bindings/SessionInput";
import type { StorageInfo } from "@/bindings/StorageInfo";
import type { SystemSettings } from "@/bindings/SystemSettings";
import type { TrackPoint } from "@/bindings/TrackPoint";
import type { TrainingEffect } from "@/bindings/TrainingEffect";
import type { TrainingPlan } from "@/bindings/TrainingPlan";
import type { TrainingNote } from "@/bindings/TrainingNote";
import type { TrainingSession } from "@/bindings/TrainingSession";
import type { User } from "@/bindings/User";
import type { UserInput } from "@/bindings/UserInput";
import type { ViewMode } from "@/bindings/ViewMode";
import type { Weather } from "@/bindings/Weather";
import type { ZonesInput } from "@/bindings/ZonesInput";
import type { ZoneTime } from "@/bindings/ZoneTime";

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
  routeHeatmap: (viewMode: ViewMode, filter?: ActivityFilter) =>
    invoke<RouteHeatmap>("route_heatmap", { viewMode, filter }),
  getActivity: (id: number) => invoke<ActivityDetail>("get_activity", { id }),
  updateActivity: (update: ActivityUpdate) => invoke<ActivityDetail>("update_activity", { update }),
  setActivityEquipment: (activityId: number, equipmentId: number | null) =>
    invoke<ActivityDetail>("set_activity_equipment", { activityId, equipmentId }),
  deleteActivity: (id: number) => invoke<void>("delete_activity", { id }),
  // Social share editor
  /** One Esri gray-canvas tile (base or labels) as image bytes. */
  mapTile: (dark: boolean, labels: boolean, z: number, x: number, y: number) =>
    invoke<ArrayBuffer>("map_tile", { dark, labels, z, x, y }),
  readImage: (path: string) => invoke<ArrayBuffer>("read_image", { path }),
  saveShareImage: (path: string, png: string) => invoke<void>("save_share_image", { path, png }),
  /** Opens the macOS share menu next to `anchor` (the button's x, y, width, height). */
  shareImage: (name: string, png: string, anchor: [number, number, number, number]) =>
    invoke<void>("share_image", { name, png, anchor }),
  activityTypeDetails: (viewMode: ViewMode) =>
    invoke<string[]>("activity_type_details", { viewMode }),

  // Detail window analyses
  activitySeries: (id: number) => invoke<TrackPoint[] | null>("activity_series", { id }),
  activitySegments: (id: number) => invoke<SegmentRow[]>("activity_segments", { id }),
  activityHeartRateZones: (id: number) =>
    invoke<ZoneTime[] | null>("activity_heart_rate_zones", { id }),
  activityTrainingEffect: (id: number) =>
    invoke<TrainingEffect>("activity_training_effect", { id }),
  fitnessTrend: (id: number) => invoke<FitnessDay[]>("fitness_trend", { id }),
  heartRateTrend: (id: number) => invoke<HeartRateTrend>("heart_rate_trend", { id }),
  /** Fitness and heart rate between two Unix times (the Health data page). */
  healthReport: (from: number, to: number) => invoke<HealthReport>("health_report", { from, to }),

  // Import and folder sync (progress arrives as `import://progress` events)
  /** Imports newest first; `range` limits by activity start. */
  importFiles: (paths: string[], range?: DateRange) =>
    invoke<ImportSummary>("import_files", { paths, range }),
  reimportActivity: (id: number) => invoke<void>("reimport_activity", { id }),
  activityRoute: (id: number) => invoke<RouteOutline | null>("activity_route", { id }),
  /** Samples and detected GPS problems, for the route fix editor. */
  activityFixSource: (id: number) => invoke<FixSource>("activity_fix_source", { id }),
  /** Replaces stretches with drawn routes and recomputes the activity. */
  applyActivityFix: (id: number, edits: FixEdit[]) =>
    invoke<void>("apply_activity_fix", { id, edits }),
  revertActivityFix: (id: number) => invoke<void>("revert_activity_fix", { id }),
  /** Imported files without GPS, waiting for their route to be drawn. */
  pendingRoutes: () => invoke<PendingRoute[]>("pending_routes"),
  pendingFixSource: (fileId: string) => invoke<FixSource>("pending_fix_source", { fileId }),
  applyPendingFix: (fileId: string, edits: FixEdit[]) =>
    invoke<ImportOutcome>("apply_pending_fix", { fileId, edits }),
  discardPendingRoute: (fileId: string) => invoke<void>("discard_pending_route", { fileId }),
  /** Writes the route as PNG or SVG, depending on the file extension. */
  exportRouteImage: (id: number, path: string) => invoke<void>("export_route_image", { id, path }),
  folderSyncStatus: () => invoke<FolderSyncStatus>("folder_sync_status"),
  setSyncFolder: (folder: string) => invoke<FolderSyncStatus>("set_sync_folder", { folder }),
  /** Stops the running import, sync or upload after the current file. */
  cancelImport: () => invoke<void>("cancel_import"),
  syncFolder: (range?: DateRange) => invoke<ImportSummary>("sync_folder", { range }),

  // Garmin Connect
  garminStatus: () => invoke<GarminStatus>("garmin_status"),
  garminSignIn: (username: string, password: string) =>
    invoke<GarminSignInResult>("garmin_sign_in", { username, password }),
  garminSubmitCode: (code: string) => invoke<void>("garmin_submit_code", { code }),
  garminSignOut: () => invoke<void>("garmin_sign_out"),
  garminSync: (range?: DateRange) => invoke<ImportSummary>("garmin_sync", { range }),
  garminExportCandidates: (since: number) =>
    invoke<ExportCandidate[]>("garmin_export_candidates", { since }),
  /** Uploads activities; progress arrives as `import://progress` events. */
  garminUpload: (ids: number[]) => invoke<ImportSummary>("garmin_upload", { ids }),
  activityLinks: (id: number) => invoke<ConnectorLink[]>("activity_links", { id }),

  // Media, weather, best performances
  listMedia: (activityId: number) => invoke<Media[]>("list_media", { activityId }),
  /** The latest photos of all activities, newest activity first. */
  recentPhotos: (limit: number) => invoke<ActivityPhoto[]>("recent_photos", { limit }),
  /** Copies the file into the media folder, or links to it when `link` is set. */
  addMedia: (activityId: number, sourcePath: string, link: boolean) =>
    invoke<Media>("add_media", { activityId, sourcePath, link }),
  /** Shows Apple's photo picker (macOS) and copies the picked items. */
  addMediaFromPhotos: (activityId: number) =>
    invoke<Media[]>("add_media_from_photos", { activityId }),
  photosAccess: () => invoke<PhotosAccess>("photos_access"),
  requestPhotosAccess: () => invoke<PhotosAccess>("request_photos_access"),
  activityPhotos: (activityId: number) => invoke<LibraryAsset[]>("activity_photos", { activityId }),
  libraryPhotoThumbnail: (id: string) => invoke<string>("library_photo_thumbnail", { id }),
  addLibraryPhotos: (activityId: number, ids: string[]) =>
    invoke<Media[]>("add_library_photos", { activityId, ids }),
  deleteMedia: (id: number) => invoke<void>("delete_media", { id }),
  getWeather: (activityId: number) => invoke<Weather | null>("get_weather", { activityId }),
  activityBestPerformances: (activityId: number) =>
    invoke<BestPerformance[]>("activity_best_performances", { activityId }),
  topBestPerformances: (viewMode: ViewMode) =>
    invoke<BestPerformance[]>("top_best_performances", { viewMode }),

  // Route segments (matching a new segment emits `segments://updated` when done)
  /** Stores the stretch between two distances (km) of the activity as a segment. */
  createSegment: (activityId: number, startKm: number, endKm: number, title: string) =>
    invoke<SegmentDetail>("create_segment", { activityId, startKm, endKm, title }),
  listSegments: (view: ViewMode, search?: string) =>
    invoke<RouteSegment[]>("list_segments", { view, search }),
  getSegment: (id: number) => invoke<SegmentDetail>("get_segment", { id }),
  renameSegment: (id: number, title: string) =>
    invoke<SegmentDetail>("rename_segment", { id, title }),
  deleteSegment: (id: number) => invoke<void>("delete_segment", { id }),
  activitySegmentEfforts: (activityId: number) =>
    invoke<ActivitySegmentEffort[]>("activity_segment_efforts", { activityId }),

  // Goals (progress and the activities that hit them are computed on read)
  /** Progress now, or at `at` (Unix timestamp). */
  goalProgress: (includeArchived: boolean, at?: number) =>
    invoke<GoalProgress[]>("goal_progress", { includeArchived, at: at ?? null }),
  createGoal: (input: GoalInput) => invoke<Goal>("create_goal", { input }),
  updateGoal: (id: number, input: GoalInput) => invoke<Goal>("update_goal", { id, input }),
  archiveGoal: (id: number, archived: boolean) => invoke<Goal>("archive_goal", { id, archived }),
  deleteGoal: (id: number) => invoke<void>("delete_goal", { id }),
  activityGoalHits: (activityId: number) => invoke<GoalHit[]>("activity_goal_hits", { activityId }),

  // Training plans (which run did which session is computed on read)
  trainingBaseline: () => invoke<Baseline>("training_baseline"),
  previewTrainingPlan: (input: PlanInput) => invoke<Preview>("preview_training_plan", { input }),
  createTrainingPlan: (input: PlanInput, createGoal: boolean) =>
    invoke<TrainingPlan>("create_training_plan", { input, createGoal }),
  trainingPlans: (includeArchived: boolean) =>
    invoke<PlanSummary[]>("training_plans", { includeArchived }),
  trainingPlan: (id: number) => invoke<PlanDetail>("training_plan", { id }),
  renameTrainingPlan: (id: number, name: string) =>
    invoke<TrainingPlan>("rename_training_plan", { id, name }),
  /** Rebuilds the sessions from today on that weren't changed by hand. */
  adaptTrainingPlan: (id: number) => invoke<PlanDetail>("adapt_training_plan", { id }),
  archiveTrainingPlan: (id: number, archived: boolean) =>
    invoke<TrainingPlan>("archive_training_plan", { id, archived }),
  deleteTrainingPlan: (id: number) => invoke<void>("delete_training_plan", { id }),
  updateTrainingSession: (id: number, input: SessionInput) =>
    invoke<TrainingSession>("update_training_session", { id, input }),
  addTrainingSession: (planId: number, input: SessionInput) =>
    invoke<TrainingSession>("add_training_session", { planId, input }),
  deleteTrainingSession: (id: number) => invoke<void>("delete_training_session", { id }),
  skipTrainingSession: (id: number, skipped: boolean) =>
    invoke<TrainingSession>("skip_training_session", { id, skipped }),
  /** Links a run by hand; `null` goes back to automatic matching unless `unlinked`. */
  linkTrainingSession: (id: number, activityId: number | null, unlinked: boolean) =>
    invoke<TrainingSession>("link_training_session", { id, activityId, unlinked }),
  trainingLinkCandidates: (id: number) => invoke<MatchedRun[]>("training_link_candidates", { id }),
  /** Sessions from `from` to `to` ("YYYY-MM-DD", both included) of the plans in use. */
  trainingSessionsBetween: (from: string, to: string) =>
    invoke<PlanSession[]>("training_sessions_between", { from, to }),
  activityTrainingSession: (activityId: number) =>
    invoke<PlanSession | null>("activity_training_session", { activityId }),

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

  // AI summaries (a local Ollama server; a finished summary also emits `activity://updated`)
  /** Models installed on the Ollama server; fails when it can't be reached. */
  aiModels: (url: string) => invoke<string[]>("ai_models", { url }),
  getAiSummary: (id: number) => invoke<AiSummary | null>("get_ai_summary", { id }),
  generateAiSummary: (id: number) => invoke<AiSummary>("generate_ai_summary", { id }),
  /** The note on a plan week (0-based) or, with `sessionId`, on a session and its run. */
  getTrainingNote: (planId: number, week: number | null, sessionId: number | null) =>
    invoke<TrainingNote | null>("get_training_note", { planId, week, sessionId }),
  generateTrainingNote: (planId: number, week: number | null, sessionId: number | null) =>
    invoke<TrainingNote>("generate_training_note", { planId, week, sessionId }),

  // System settings
  getSettings: () => invoke<SystemSettings>("get_settings"),
  saveSettings: (settings: SystemSettings) => invoke<SystemSettings>("save_settings", { settings }),
  storageInfo: () => invoke<StorageInfo>("storage_info"),
};
