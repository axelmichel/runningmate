import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api";

/** Every plan with its progress, archived ones only when asked for. */
export function useTrainingPlans(includeArchived = false) {
  return useQuery({
    queryKey: ["training", "plans", includeArchived],
    queryFn: () => api.trainingPlans(includeArchived),
  });
}

export function useTrainingPlan(id: number | null) {
  return useQuery({
    queryKey: ["training", "plan", id],
    queryFn: () => api.trainingPlan(id!),
    enabled: id !== null,
  });
}

/** Sessions from `from` to `to` ("YYYY-MM-DD") of the plans in use. */
export function useTrainingSessions(from: string, to: string) {
  return useQuery({
    queryKey: ["training", "sessions", from, to],
    queryFn: () => api.trainingSessionsBetween(from, to),
  });
}

/** The session an activity did, loaded while `enabled`. */
export function useActivitySession(activityId: number, enabled = true) {
  return useQuery({
    queryKey: ["training", "activity", activityId],
    queryFn: () => api.activityTrainingSession(activityId),
    enabled,
  });
}
