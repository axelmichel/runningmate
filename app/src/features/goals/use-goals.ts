import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api";

/** Every goal with its progress, archived ones only when asked for; `at` for a past moment. */
export function useGoalProgress(includeArchived = false, at?: number) {
  return useQuery({
    queryKey: ["goals", "progress", includeArchived, at],
    queryFn: () => api.goalProgress(includeArchived, at),
  });
}

/** The goals an activity hit, loaded while `enabled`. */
export function useGoalHits(activityId: number, enabled = true) {
  return useQuery({
    queryKey: ["goalHits", activityId],
    queryFn: () => api.activityGoalHits(activityId),
    enabled,
  });
}
