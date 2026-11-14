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
  });
});
