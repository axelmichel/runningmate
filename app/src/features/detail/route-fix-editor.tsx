import "leaflet/dist/leaflet.css";

import { divIcon, type LatLngTuple } from "leaflet";
import { Check, MousePointerClick, Trash2, Undo2, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { CircleMarker, MapContainer, Marker, Polyline, useMap, useMapEvents } from "react-leaflet";
import { toast } from "sonner";

import type { FixEdit } from "@/bindings/FixEdit";
import type { FixIssue } from "@/bindings/FixIssue";
import type { FixIssueKind } from "@/bindings/FixIssueKind";
import type { FixSource } from "@/bindings/FixSource";
import { Button } from "@/components/ui/button";
import { boundsOf, FINISH_COLOR, START_COLOR } from "@/features/segments/segment-math";
import { t, tk } from "@/i18n";
import { formatDuration, formatNumber } from "@/lib/format";
import { cn } from "@/lib/utils";

import { Basemap, MapCredit } from "./basemap";
import {
  editedDistance,
  editPath,
  nearestSample,
  pathLength,
  positionsBetween,
  toEdit,
} from "./fix-math";

const TRACK_COLOR = "#9aa3ad";
const ISSUE_COLOR = "#e13342";
const SELECTION_COLOR = "#2f80ed";
const FIXED_COLOR = "#27ae60";
/** How far (screen pixels) from the track a click may be when picking a stretch. */
const SNAP_PIXELS = 24;
/** Shown for an activity without any GPS until the first point is drawn. */
const FALLBACK_CENTER: LatLngTuple = [50, 10];

const ISSUE_LABELS: Record<FixIssueKind, string> = {
  spike: tk("GPS spike"),
  gap: tk("GPS dropout"),
  noGps: tk("No GPS"),
};

type Draft = { start: number; end: number | null; vertices: LatLngTuple[] };

const vertexIcon = divIcon({
  className: "",
  html: `<div style="width:12px;height:12px;border-radius:9999px;background:#fff;border:3px solid ${SELECTION_COLOR}"></div>`,
  iconSize: [12, 12],
  iconAnchor: [6, 6],
});

/** Keeps Leaflet's size in sync when the surrounding layout changes width. */
function FitToContainer() {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => map.invalidateSize());
    observer.observe(map.getContainer());
    return () => observer.disconnect();
  }, [map]);
  return null;
}

/**
 * Map clicks: while picking, the first click near the track sets the start of the stretch and
 * the second its end; while drawing, every click adds a point to the route.
 */
function ClickHandler({
  source,
  draft,
  picking,
  onPick,
  onDraw,
}: {
  source: FixSource;
  draft: Draft | null;
  picking: boolean;
  onPick: (index: number) => void;
  onDraw: (position: LatLngTuple) => void;
}) {
  const map = useMapEvents({
    click: ({ latlng, containerPoint }) => {
      if (picking) {
        const i = nearestSample(
          source.samples,
          (p) => map.latLngToContainerPoint(p).distanceTo(containerPoint),
          SNAP_PIXELS,
        );
        if (i === null) toast.info(t("Click closer to the track."));
        else onPick(i);
      } else if (draft?.end != null) {
        onDraw([latlng.lat, latlng.lng]);
      }
    },
  });
  return null;
}

function seconds(source: FixSource, index: number) {
  return (source.samples[index].time - source.samples[0].time) / 1000;
}

function IssueButton({
  source,
  issue,
  active,
  onClick,
}: {
  source: FixSource;
  issue: FixIssue;
  active: boolean;
  onClick: () => void;
}) {
  const length = seconds(source, issue.end) - seconds(source, issue.start);
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-center justify-between gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted",
        active && "bg-muted",
      )}
    >
      <span className="flex items-center gap-2">
        <span className="size-2 shrink-0 rounded-full" style={{ background: ISSUE_COLOR }} />
        {t(ISSUE_LABELS[issue.kind])}
      </span>
      <span className="text-xs text-muted-foreground tabular-nums">
        {formatDuration(seconds(source, issue.start))} · {formatDuration(length)}
      </span>
    </button>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {title}
      </h3>
      {children}
    </section>
  );
}

/**
 * Redraws broken stretches of a track by hand: pick a detected problem or any stretch of the
 * track, click the route on the map (points can be dragged), and add it as a fix. `onSave`
 * gets all fixes; the backend moves the samples onto the drawn routes and recomputes.
 */
export function RouteFixEditor({
  source,
  saving,
  onSave,
  extraActions,
  hideMap = false,
}: {
  source: FixSource;
  saving: boolean;
  onSave: (edits: FixEdit[]) => void;
  /** Further buttons below Save, such as resetting to the original. */
  extraActions?: React.ReactNode;
  /** Hides the map while a dialog is open above it (WebKit paints tiles above dialogs). */
  hideMap?: boolean;
}) {
  const { samples, issues } = source;
  // Without any GPS there is only one thing to do: draw the whole route.
  const [draft, setDraft] = useState<Draft | null>(() => {
    const whole = issues.find((issue) => issue.kind === "noGps");
    return whole ? { start: whole.start, end: whole.end, vertices: [] } : null;
  });
  const [picking, setPicking] = useState(false);
  const [edits, setEdits] = useState<FixEdit[]>([]);
  const dark = document.documentElement.classList.contains("dark");

  const track = useMemo(() => positionsBetween(samples), [samples]);
  const bounds = useMemo(() => (track.length >= 2 ? boundsOf(track) : null), [track]);
  const recorded = useMemo(() => pathLength(track), [track]);
  const draftPath =
    draft?.end != null ? editPath(samples, draft.start, draft.end, draft.vertices) : [];
  const after = editedDistance(samples, edits);

  const startPicking = () => {
    setDraft(null);
    setPicking(true);
  };
  const pick = (index: number) => {
    if (draft === null || draft.end !== null) {
      setDraft({ start: index, end: null, vertices: [] });
      return;
    }
    if (index === draft.start) return;
    const [start, end] = [draft.start, index].sort((a, b) => a - b);
    setDraft({ start, end, vertices: [] });
    setPicking(false);
  };
  const selectIssue = (issue: FixIssue) => {
    setPicking(false);
    setDraft({ start: issue.start, end: issue.end, vertices: [] });
  };
  const setVertices = (update: (vertices: LatLngTuple[]) => LatLngTuple[]) =>
    setDraft((d) => (d ? { ...d, vertices: update(d.vertices) } : d));
  const addDraft = () => {
    if (draft?.end == null || draftPath.length < 2) return;
    setEdits([...edits, toEdit(samples, draft.start, draft.end, draftPath)]);
    setDraft(null);
  };
  const cancel = () => {
    setDraft(null);
    setPicking(false);
  };

  return (
    <div className="flex h-full min-h-0 gap-4">
      <div
        className={cn(
          "relative isolate min-h-72 flex-1 overflow-hidden rounded-lg border",
          (picking || draft?.end != null) && "[&_.leaflet-container]:cursor-crosshair",
          hideMap && "[&_.leaflet-container]:invisible",
        )}
      >
        <MapContainer
          {...(bounds
            ? { bounds, boundsOptions: { padding: [24, 24] } }
            : source.center
              ? { center: source.center, zoom: 14 }
              : { center: FALLBACK_CENTER, zoom: 5 })}
          className="h-full w-full"
          attributionControl={false}
          doubleClickZoom={false}
        >
          <Basemap dark={dark} />
          <FitToContainer />
          <Polyline
            positions={track}
            pathOptions={{ color: TRACK_COLOR, weight: 4, opacity: 0.9 }}
          />
          {issues.map((issue, i) => (
            <Polyline
              key={i}
              positions={positionsBetween(samples, issue.start, issue.end)}
              pathOptions={{ color: ISSUE_COLOR, weight: 5, opacity: 0.9 }}
              eventHandlers={{ click: () => !picking && selectIssue(issue) }}
            />
          ))}
          {edits.map((edit, i) => (
            <Polyline
              key={i}
              positions={edit.path}
              pathOptions={{ color: FIXED_COLOR, weight: 5, opacity: 0.95 }}
            />
          ))}
          {draft?.end != null && (
            <Polyline
              positions={positionsBetween(samples, draft.start, draft.end)}
              pathOptions={{ color: SELECTION_COLOR, weight: 3, opacity: 0.6, dashArray: "4 6" }}
            />
          )}
          {draftPath.length >= 2 && (
            <Polyline
              positions={draftPath}
              pathOptions={{ color: SELECTION_COLOR, weight: 5, opacity: 0.95 }}
            />
          )}
          {draft && positionsBetween(samples, draft.start, draft.start)[0] && (
            <CircleMarker
              center={positionsBetween(samples, draft.start, draft.start)[0]}
              radius={7}
              pathOptions={{ color: "#fff", weight: 2, fillColor: START_COLOR, fillOpacity: 1 }}
            />
          )}
          {draft?.end != null && positionsBetween(samples, draft.end, draft.end)[0] && (
            <CircleMarker
              center={positionsBetween(samples, draft.end, draft.end)[0]}
              radius={7}
              pathOptions={{ color: "#fff", weight: 2, fillColor: FINISH_COLOR, fillOpacity: 1 }}
            />
          )}
          {draft?.vertices.map((vertex, i) => (
            <Marker
              key={i}
              position={vertex}
              icon={vertexIcon}
              draggable
              eventHandlers={{
                dragend: (event) => {
                  const { lat, lng } = event.target.getLatLng();
                  setVertices((vertices) => vertices.map((v, j) => (j === i ? [lat, lng] : v)));
                },
              }}
            />
          ))}
          <ClickHandler
            source={source}
            draft={draft}
            picking={picking}
            onPick={pick}
            onDraw={(position) => setVertices((vertices) => [...vertices, position])}
          />
        </MapContainer>
        <MapCredit />
      </div>

      <aside className="flex w-72 shrink-0 flex-col gap-5 overflow-y-auto">
        <Section title={t("Problems found")}>
          {issues.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("No GPS problems found.")}</p>
          ) : (
            <div className="flex flex-col">
              {issues.map((issue, i) => (
                <IssueButton
                  key={i}
                  source={source}
                  issue={issue}
                  active={draft?.start === issue.start && draft.end === issue.end}
                  onClick={() => selectIssue(issue)}
                />
              ))}
            </div>
          )}
          {track.length >= 2 && (
            <Button size="sm" variant="outline" onClick={startPicking} disabled={picking}>
              <MousePointerClick />
              {t("Pick a stretch on the map")}
            </Button>
          )}
        </Section>

        {(picking || draft) && (
          <Section title={t("Redraw")}>
            <p className="text-sm text-muted-foreground">
              {picking
                ? draft
                  ? t("Click where the stretch ends.")
                  : t("Click where the stretch starts.")
                : t("Click the route on the map; drag points to move them.")}
            </p>
            {draft?.end != null && (
              <p className="text-sm tabular-nums">
                {formatDuration(seconds(source, draft.start))} –{" "}
                {formatDuration(seconds(source, draft.end))} ·{" "}
                {formatNumber(pathLength(draftPath) / 1000, 2)} km
              </p>
            )}
            <div className="flex flex-wrap gap-2">
              {draft?.end != null && (
                <>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setVertices((vertices) => vertices.slice(0, -1))}
                    disabled={draft.vertices.length === 0}
                  >
                    <Undo2 />
                    {t("Undo point")}
                  </Button>
                  <Button size="sm" onClick={addDraft} disabled={draftPath.length < 2}>
                    <Check />
                    {t("Use route")}
                  </Button>
                </>
              )}
              <Button size="sm" variant="ghost" onClick={cancel}>
                <X />
                {t("Cancel")}
              </Button>
            </div>
          </Section>
        )}

        <Section title={t("Fixes")}>
          {edits.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("No fixes yet.")}</p>
          ) : (
            <ul className="flex flex-col gap-1">
              {edits.map((edit, i) => (
                <li
                  key={i}
                  className="flex items-center justify-between gap-2 text-sm tabular-nums"
                >
                  <span className="flex items-center gap-2">
                    <span className="size-2 rounded-full" style={{ background: FIXED_COLOR }} />
                    {formatDuration((edit.from - samples[0].time) / 1000)} –{" "}
                    {formatDuration((edit.to - samples[0].time) / 1000)}
                  </span>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="size-7"
                    aria-label={t("Remove")}
                    onClick={() => setEdits(edits.filter((_, j) => j !== i))}
                  >
                    <Trash2 />
                  </Button>
                </li>
              ))}
            </ul>
          )}
          <dl className="grid grid-cols-2 gap-x-3 gap-y-1 text-sm tabular-nums">
            <dt className="text-muted-foreground">{t("Recorded")}</dt>
            <dd className="text-right">{formatNumber(recorded / 1000, 2)} km</dd>
            <dt className="text-muted-foreground">{t("After the fixes")}</dt>
            <dd className="text-right font-semibold">{formatNumber(after / 1000, 2)} km</dd>
          </dl>
          <Button onClick={() => onSave(edits)} disabled={edits.length === 0 || saving}>
            {saving ? t("Saving…") : t("Save and recompute")}
          </Button>
          {extraActions}
        </Section>
      </aside>
    </div>
  );
}
