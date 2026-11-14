import { invoke } from "@tauri-apps/api/core";

export type AppInfo = {
  version: string;
  dataDir: string;
};

export const api = {
  appInfo: () => invoke<AppInfo>("app_info"),
};
