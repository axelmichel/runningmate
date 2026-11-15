import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

const { api } = await import("./api");

describe("api", () => {
  beforeEach(() => invoke.mockReset().mockResolvedValue(undefined));

  it("passes arguments under the names the Rust commands expect", async () => {
    await api.listActivities({
      viewMode: "Running",
      offset: 25,
      limit: 25,
      sortField: "date",
      sortDirection: "desc",
      filter: { searchText: "hill" },
    });
    expect(invoke).toHaveBeenCalledWith("list_activities", {
      query: expect.objectContaining({ viewMode: "Running", filter: { searchText: "hill" } }),
    });

    await api.setActivityEquipment(4, null);
    expect(invoke).toHaveBeenLastCalledWith("set_activity_equipment", {
      activityId: 4,
      equipmentId: null,
    });

    await api.addEquipment("bike", "Gravel", 10.5, true);
    expect(invoke).toHaveBeenLastCalledWith("add_equipment", {
      kind: "bike",
      name: "Gravel",
      weight: 10.5,
      inUse: true,
    });

    await api.aiModels("http://localhost:11434");
    expect(invoke).toHaveBeenLastCalledWith("ai_models", { url: "http://localhost:11434" });

    await api.generateAiSummary(7);
    expect(invoke).toHaveBeenLastCalledWith("generate_ai_summary", { id: 7 });

    await api.archiveGoal(3, true);
    expect(invoke).toHaveBeenLastCalledWith("archive_goal", { id: 3, archived: true });

    await api.activityGoalHits(9);
    expect(invoke).toHaveBeenLastCalledWith("activity_goal_hits", { activityId: 9 });
  });
});
