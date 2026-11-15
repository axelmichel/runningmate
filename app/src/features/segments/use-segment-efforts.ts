import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api";

/** The passes of an activity over segments, loaded while `enabled`. */
export function useSegmentEfforts(activityId: number, enabled = true) {
  return useQuery({
    queryKey: ["segmentEfforts", activityId],
    queryFn: () => api.activitySegmentEfforts(activityId),
    enabled,
  });
}
