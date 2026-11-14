import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api";

/**
 * Track points of an activity, parsed from its archived TCX file once per session; not loaded
 * while `enabled` is false.
 */
export function useSeries(activityId: number, enabled = true) {
  return useQuery({
    queryKey: ["series", activityId],
    queryFn: () => api.activitySeries(activityId),
    staleTime: Infinity,
    enabled,
  });
}
