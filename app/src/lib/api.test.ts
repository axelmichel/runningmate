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

    await api.createTrainingPlan(
      {
        name: "10k",
        target: "race",
        distance: 10,
        targetTime: null,
        start: "2026-11-16",
        weeks: 8,
        days: [1, 3, 6],
        longDay: 6,
      },
      true,
    );
    expect(invoke).toHaveBeenLastCalledWith("create_training_plan", {
      input: expect.objectContaining({ longDay: 6 }),
      createGoal: true,
    });

    await api.linkTrainingSession(5, null, true);
    expect(invoke).toHaveBeenLastCalledWith("link_training_session", {
      id: 5,
      activityId: null,
      unlinked: true,
    });

    await api.trainingSessionsBetween("2026-11-16", "2026-11-29");
    expect(invoke).toHaveBeenLastCalledWith("training_sessions_between", {
      from: "2026-11-16",
      to: "2026-11-29",
    });
  });
});
