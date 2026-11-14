import { useQuery } from "@tanstack/react-query";

import { api } from "@/lib/api";

/** Track points of an activity, parsed from its archived TCX file once per session. */
export function useSeries(activityId: number) {
  return useQuery({
    queryKey: ["series", activityId],
    queryFn: () => api.activitySeries(activityId),
    staleTime: Infinity,
  });
}
