import { useQuery } from "@tanstack/react-query";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  AlignCenterHorizontal,
  AlignCenterVertical,
  AlignEndHorizontal,
  AlignEndVertical,
  AlignHorizontalSpaceAround,
  AlignStartHorizontal,
  AlignStartVertical,
  AlignVerticalSpaceAround,
  Group,
  ImagePlus,
  Loader2,
  RotateCcw,
  Save,
  Share,
  Ungroup,
  type LucideIcon,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { toast } from "sonner";

import type { ActivityDetail } from "@/bindings/ActivityDetail";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { useSeries } from "@/features/detail/use-series";
import { t, tk } from "@/i18n";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";

import {
  align,
  applyMoves,
  distribute,
  groupOf,
  snapMove,
  unionBox,
  unitsOf,
  type Alignment,
  type Axis,
  type Moves,
} from "./share-arrange";
import { drawShareImage, type Box, type Layout } from "./share-render";
import {
  DEFAULT_POSITIONS,
  DEFAULT_SCENE,
  GRID_DIVISIONS,
  MAX_SIZE,
  MIN_SIZE,
  SIZE_PRESETS,
  STAT_KEYS,
  STAT_LABELS,
  gridLines,
  gridStep,
  infoItems,
  routeOf,
  shareFileName,
  type BackgroundKind,
  type ElementId,
  type Scene,
} from "./share-scene";
import { useBitmaps } from "./use-bitmaps";

const STORAGE_KEY = "runningmate-share-scene";

const BACKGROUNDS: { value: BackgroundKind; label: string }[] = [
  { value: "gradient", label: tk("Gradient") },
  { value: "color", label: tk("Single colour") },
  { value: "image", label: tk("Photo") },
  { value: "map", label: tk("Map") },
  { value: "transparent", label: tk("Transparent") },
];

/** The last settings, so the next image starts from them (the photo is per activity). */
function storedScene(): Scene {
  try {
    const stored = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "null") as Partial<Scene>;
    return stored
      ? {
          ...DEFAULT_SCENE,
          ...stored,
          route: { ...DEFAULT_SCENE.route, ...stored.route },
          info: {
            ...DEFAULT_SCENE.info,
            ...stored.info,
            positions: { ...DEFAULT_POSITIONS, ...stored.info?.positions },
          },
          groups: stored.groups ?? [],
          grid: { ...DEFAULT_SCENE.grid, ...stored.grid },
        }
      : DEFAULT_SCENE;
  } catch {
    return DEFAULT_SCENE;
  }
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="grid gap-3 border-b pb-4 last:border-b-0">
      <h3 className="text-xs font-semibold tracking-wide text-muted-foreground uppercase">
        {title}
      </h3>
      {children}
    </section>
  );
}

function Range({
  label,
  value,
  min,
  max,
  step,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (value: number) => void;
}) {
  return (
    <div className="grid gap-2">
      <Label className="font-normal">{label}</Label>
      <Slider
        min={min}
        max={max}
        step={step}
        value={[value]}
        onValueChange={([next]) => onChange(next)}
        aria-label={label}
      />
    </div>
  );
}

function ColorField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-3 text-sm">
      {label}
      <input
        type="color"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        className="h-7 w-12 cursor-pointer rounded border bg-transparent"
      />
    </label>
  );
}

/** Pixel size, applied on leaving the field or Enter (clamping while typing would fight it). */
function SizeInput({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState<string | null>(null);
  const commit = () => {
    const size = Number(draft);
    if (draft !== null && Number.isFinite(size) && size > 0) {
      onChange(Math.round(Math.min(MAX_SIZE, Math.max(MIN_SIZE, size))));
    }
    setDraft(null);
  };
  return (
    <Input
      type="number"
      aria-label={label}
      min={MIN_SIZE}
      max={MAX_SIZE}
      value={draft ?? value}
      onChange={(event) => setDraft(event.target.value)}
      onBlur={commit}
      onKeyDown={(event) => event.key === "Enter" && commit()}
    />
  );
}

/** Arranging the selection: alignment, equal spacing and grouping. */
function ArrangeBar({
  units,
  selected,
  grouped,
  onAlign,
  onDistribute,
  onGroup,
  onUngroup,
}: {
  units: ElementId[][];
  selected: number;
  grouped: boolean;
  onAlign: (alignment: Alignment) => void;
  onDistribute: (axis: Axis) => void;
  onGroup: () => void;
  onUngroup: () => void;
}) {
  // One unit lines up on the image, several with each other.
  const onImage = units.length === 1;
  const tool = (label: string, Icon: LucideIcon, onClick: () => void, disabled = false) => (
    <Button
      key={label}
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      title={label}
      disabled={disabled}
      onClick={onClick}
    >
      <Icon />
    </Button>
  );
  const none = units.length === 0;
  return (
    <div className="flex flex-wrap items-center gap-1 rounded-lg border px-1.5 py-1">
      {tool(
        onImage ? t("Left edge of the image") : t("Align left edges"),
        AlignStartVertical,
        () => onAlign("left"),
        none,
      )}
      {tool(
        onImage ? t("Centre horizontally on the image") : t("Align centres horizontally"),
        AlignCenterVertical,
        () => onAlign("centre"),
        none,
      )}
      {tool(
        onImage ? t("Right edge of the image") : t("Align right edges"),
        AlignEndVertical,
        () => onAlign("right"),
        none,
      )}
      <span className="mx-1 h-5 w-px bg-border" />
      {tool(
        onImage ? t("Top edge of the image") : t("Align top edges"),
        AlignStartHorizontal,
        () => onAlign("top"),
        none,
      )}
      {tool(
        onImage ? t("Centre vertically on the image") : t("Align centres vertically"),
        AlignCenterHorizontal,
        () => onAlign("middle"),
        none,
      )}
      {tool(
        onImage ? t("Bottom edge of the image") : t("Align bottom edges"),
        AlignEndHorizontal,
        () => onAlign("bottom"),
        none,
      )}
      <span className="mx-1 h-5 w-px bg-border" />
      {tool(
        t("Equal space horizontally"),
        AlignHorizontalSpaceAround,
        () => onDistribute("horizontal"),
        none,
      )}
      {tool(
        t("Equal space vertically"),
        AlignVerticalSpaceAround,
        () => onDistribute("vertical"),
        none,
      )}
      <span className="mx-1 h-5 w-px bg-border" />
      {tool(t("Group"), Group, onGroup, selected < 2 || (units.length === 1 && grouped))}
      {tool(t("Ungroup"), Ungroup, onUngroup, !grouped)}
      <span className="ml-auto pr-1 text-xs text-muted-foreground">
        {none ? t("Nothing selected") : t("{{count}} selected", { count: selected })}
      </span>
    </div>
  );
}

/**
 * Editing marks on top of the preview, never in the image: the grid (centre lines stronger),
 * outlines of the elements while the grid is shown, and the selection.
 */
function Overlay({
  scene,
  layout,
  selection,
}: {
  scene: Scene;
  layout: Layout;
  selection: ElementId[];
}) {
  const step = gridStep(scene);
  const stroke = Math.min(scene.width, scene.height) / 500;
  const line = (at: number, centre: number) => (Math.abs(at - centre) < 1e-6 ? 0.9 : 0.45);
  const rect = (box: Box, key: string, props: React.SVGProps<SVGRectElement>) => (
    <rect
      key={key}
      x={box.left}
      y={box.top}
      width={box.width}
      height={box.height}
      fill="none"
      {...props}
    />
  );
  const groups = scene.groups
    .map((group) => unionBox(group, layout))
    .filter((box): box is Box => box !== null);
  return (
    <svg
      className="pointer-events-none absolute inset-0 size-full"
      viewBox={`0 0 ${scene.width} ${scene.height}`}
      preserveAspectRatio="none"
      aria-hidden
    >
      {scene.grid.show && (
        <>
          {gridLines(scene.width, step).map((x) => (
            <line
              key={`x${x}`}
              x1={x}
              x2={x}
              y1={0}
              y2={scene.height}
              stroke="#38bdf8"
              strokeOpacity={line(x, scene.width / 2)}
              strokeWidth={stroke}
            />
          ))}
          {gridLines(scene.height, step).map((y) => (
            <line
              key={`y${y}`}
              y1={y}
              y2={y}
              x1={0}
              x2={scene.width}
              stroke="#38bdf8"
              strokeOpacity={line(y, scene.height / 2)}
              strokeWidth={stroke}
            />
          ))}
          {(Object.entries(layout) as [ElementId, Box][]).map(([id, box]) =>
            rect(box, `outline-${id}`, {
              stroke: "#38bdf8",
              strokeOpacity: 0.7,
              strokeWidth: stroke,
              strokeDasharray: `${stroke * 6} ${stroke * 6}`,
            }),
          )}
        </>
      )}
      {groups.map((box, i) =>
        rect(box, `group-${i}`, {
          stroke: "#a855f7",
          strokeWidth: stroke * 1.5,
          strokeDasharray: `${stroke * 10} ${stroke * 5}`,
        }),
      )}
      {selection.map((id) => {
        const box = layout[id];
        return box && rect(box, `selected-${id}`, { stroke: "#f97316", strokeWidth: stroke * 2.5 });
      })}
    </svg>
  );
}

/**
 * Builds an image for social media from the activity: the route, chosen facts and a
 * background, in a platform's size. The preview is the image itself, scaled down; drag the
 * route or the facts to place them.
 */
export function ShareTab({ activity }: { activity: ActivityDetail }) {
  const [scene, setScene] = useState(storedScene);
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [fontsReady, setFontsReady] = useState(false);
  const [complete, setComplete] = useState(true);
  const [busy, setBusy] = useState(false);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const layoutRef = useRef<Layout>({});
  /** The drag in progress: where it started, the scene then and the moved elements' box. */
  const dragRef = useRef<{
    x: number;
    y: number;
    from: Scene;
    ids: ElementId[];
    box: Box | null;
  } | null>(null);
  const [selection, setSelection] = useState<ElementId[]>([]);
  const shareRef = useRef<HTMLButtonElement>(null);
  const stageRef = useRef<HTMLDivElement>(null);
  const [stage, setStage] = useState({ width: 0, height: 0 });
  const [layout, setLayout] = useState<Layout>({});
  const { bitmap, version } = useBitmaps();

  const { data: points } = useSeries(activity.id);
  const route = useMemo(() => (points ? routeOf(points) : null), [points]);
  const items = useMemo(
    () => infoItems(activity, scene.info.fields),
    [activity, scene.info.fields],
  );
  const { data: media = [] } = useQuery({
    queryKey: ["media", activity.id],
    queryFn: () => api.listMedia(activity.id),
  });
  const photos = media.filter((m) => m.mediaType === "image" && !m.missing);
  const photo = imagePath ?? photos[0]?.filePath ?? null;

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(scene));
  }, [scene]);
  useEffect(() => {
    void document.fonts.load('600 16px "Geist Variable"').finally(() => setFontsReady(true));
  }, []);

  // Drawn again whenever the scene changes or another map tile or the photo arrives.
  useEffect(() => {
    const ctx = canvasRef.current?.getContext("2d");
    if (!ctx || !fontsReady) return;
    let missing = false;
    const track = <T,>(image: T | undefined) => {
      if (image === undefined) missing = true;
      return image;
    };
    layoutRef.current = drawShareImage(ctx, scene, route, items, {
      tile: (tile, labels) =>
        track(
          bitmap(`tile:${scene.mapDark}:${labels}:${tile.z}/${tile.x}/${tile.y}`, () =>
            api.mapTile(scene.mapDark, labels, tile.z, tile.x, tile.y),
          ),
        ),
      background:
        scene.background === "image" && photo
          ? track(bitmap(`photo:${photo}`, () => api.readImage(photo)))
          : undefined,
    });
    setComplete(!missing);
    setLayout((current) =>
      JSON.stringify(current) === JSON.stringify(layoutRef.current) ? current : layoutRef.current,
    );
  }, [fontsReady, scene, route, items, photo, bitmap, version]);

  useEffect(() => {
    const element = stageRef.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) =>
      setStage({ width: entry.contentRect.width, height: entry.contentRect.height }),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  // The image scaled to fit the stage with a small margin, never larger than it is.
  const fit = Math.min(1, (stage.width - 24) / scene.width, (stage.height - 24) / scene.height);
  const display = {
    width: Math.max(0, scene.width * fit),
    height: Math.max(0, scene.height * fit),
  };

  const set = (patch: Partial<Scene>) => setScene((current) => ({ ...current, ...patch }));
  const setRoute = (patch: Partial<Scene["route"]>) =>
    setScene((current) => ({ ...current, route: { ...current.route, ...patch } }));
  const setInfo = (patch: Partial<Scene["info"]>) =>
    setScene((current) => ({ ...current, info: { ...current.info, ...patch } }));

  const preset =
    SIZE_PRESETS.find((p) => p.width === scene.width && p.height === scene.height)?.id ?? "custom";

  /** Pointer position in image pixels. */
  const at = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    return [
      ((event.clientX - rect.left) / rect.width) * scene.width,
      ((event.clientY - rect.top) / rect.height) * scene.height,
    ];
  };
  const size = { width: scene.width, height: scene.height };
  // Hidden elements (unticked facts, a hidden route) drop out of the selection.
  const selected = selection.filter((id) => layout[id]);
  const units = unitsOf(selected, scene.groups);

  /** Topmost element under a point: facts lie above the route, later ones above earlier. */
  const elementAt = (x: number, y: number): ElementId | null => {
    const pad = Math.min(scene.width, scene.height) * 0.015;
    const hit = (box: Box | undefined) =>
      !!box &&
      x >= box.left - pad &&
      x <= box.left + box.width + pad &&
      y >= box.top - pad &&
      y <= box.top + box.height + pad;
    const facts = (Object.keys(layoutRef.current) as ElementId[]).filter((id) => id !== "route");
    return (
      facts.reverse().find((id) => hit(layoutRef.current[id])) ??
      (hit(layoutRef.current.route) ? "route" : null)
    );
  };

  /** Moves elements from where they were in `from` and keeps them selected. */
  const move = (from: Scene, moves: Moves) =>
    setScene((current) => applyMoves(current, from, moves));

  const onPointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const [x, y] = at(event);
    const element = elementAt(x, y);
    if (!element) {
      if (!event.shiftKey) setSelection([]);
      return;
    }
    const group = groupOf(element, scene.groups);
    let next: ElementId[];
    if (event.shiftKey) {
      // Shift-click adds the element (with its group) or takes it out again.
      next = selected.includes(element)
        ? selected.filter((id) => !group.includes(id))
        : [...selected, ...group.filter((id) => !selected.includes(id))];
      setSelection(next);
      if (!next.includes(element)) return;
    } else {
      next = selected.includes(element) ? selected : group;
      setSelection(next);
    }
    const ids = next.filter((id) => layoutRef.current[id]);
    dragRef.current = { x, y, from: scene, ids, box: unionBox(ids, layoutRef.current) };
    event.currentTarget.setPointerCapture(event.pointerId);
  };
  const onPointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const drag = dragRef.current;
    if (!drag) return;
    const [x, y] = at(event);
    let [dx, dy] = [x - drag.x, y - drag.y];
    // The moved elements together sit on the grid, unless Option is held.
    if (scene.grid.show && !event.altKey && drag.box) {
      [dx, dy] = snapMove(drag.box, dx, dy, size, gridStep(scene));
    }
    move(drag.from, Object.fromEntries(drag.ids.map((id) => [id, [dx, dy]])));
  };
  const onPointerUp = () => {
    dragRef.current = null;
  };
  // Arrow keys nudge the selection: a pixel, ten with Shift, a grid square while it's shown.
  const onKeyDown = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const arrows: Record<string, [number, number]> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    if (event.key === "Escape") setSelection([]);
    const arrow = arrows[event.key];
    if (!arrow || !selected.length) return;
    event.preventDefault();
    const by = scene.grid.show ? gridStep(scene) : event.shiftKey ? 10 : 1;
    move(scene, Object.fromEntries(selected.map((id) => [id, [arrow[0] * by, arrow[1] * by]])));
  };

  const arrange = (moves: Moves) => move(scene, moves);
  const group = () =>
    set({
      // An element is in one group at most: the new group takes it from any other.
      groups: [
        ...scene.groups
          .map((g) => g.filter((id) => !selected.includes(id)))
          .filter((g) => g.length > 1),
        selected,
      ],
    });
  const ungroup = () =>
    set({ groups: scene.groups.filter((g) => !g.some((id) => selected.includes(id))) });
  const grouped = scene.groups.some((g) => g.some((id) => selected.includes(id)));

  const png = () => canvasRef.current!.toDataURL("image/png").split(",")[1];
  const saveImage = async () => {
    const path = await save({
      defaultPath: `${shareFileName(activity)}.png`,
      filters: [{ name: "PNG", extensions: ["png"] }],
    });
    if (!path) return;
    setBusy(true);
    try {
      await api.saveShareImage(path, png());
      toast.success(t("Image saved"));
    } catch (error) {
      toast.error(t("Could not save the image: {{error}}", { error: String(error) }));
    } finally {
      setBusy(false);
    }
  };
  const share = async () => {
    const rect = shareRef.current!.getBoundingClientRect();
    setBusy(true);
    try {
      await api.shareImage(shareFileName(activity), png(), [
        rect.left,
        rect.top,
        rect.width,
        rect.height,
      ]);
    } catch (error) {
      toast.error(t("Could not share the image: {{error}}", { error: String(error) }));
    } finally {
      setBusy(false);
    }
  };
  const chooseImage = async () => {
    const path = await open({
      multiple: false,
      filters: [{ name: t("Images"), extensions: ["jpg", "jpeg", "png", "heic"] }],
    });
    if (typeof path === "string") setImagePath(path);
  };

  return (
    <div className="flex h-full min-h-[32rem] gap-4">
      <div className="flex min-w-0 flex-1 flex-col gap-3">
        <ArrangeBar
          units={units}
          selected={selected.length}
          grouped={grouped}
          onAlign={(alignment) => arrange(align(units, layout, alignment, size))}
          onDistribute={(axis) => arrange(distribute(units, layout, axis, size))}
          onGroup={group}
          onUngroup={ungroup}
        />
        <div
          ref={stageRef}
          className="relative flex min-h-0 flex-1 items-center justify-center overflow-hidden rounded-lg border bg-muted/40"
        >
          {/* Sized in script, so the overlay covers exactly the scaled-down image. Focusable,
              for the arrow keys. */}
          <div
            tabIndex={0}
            onKeyDown={onKeyDown}
            className="relative shadow-md outline-none"
            style={{ width: display.width, height: display.height }}
          >
            <canvas
              ref={canvasRef}
              width={scene.width}
              height={scene.height}
              aria-label={t("Share image preview")}
              // A checkerboard shows through where the image is transparent.
              className="block size-full touch-none bg-[conic-gradient(#ccc_25%,#fff_0_50%,#ccc_0_75%,#fff_0)] bg-[length:16px_16px]"
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
              onPointerCancel={onPointerUp}
            />
            <Overlay scene={scene} layout={layout} selection={selected} />
          </div>
        </div>
        <div className="flex flex-wrap items-center justify-between gap-2">
          <p className="text-xs text-muted-foreground">
            {complete
              ? t("Click to select, Shift-click to add more; drag to move, arrow keys to nudge.")
              : t("Loading the map and photo…")}
          </p>
          <div className="flex gap-2">
            <Button variant="outline" disabled={busy || !complete} onClick={() => void saveImage()}>
              <Save />
              {t("Save image…")}
            </Button>
            <Button ref={shareRef} disabled={busy || !complete} onClick={() => void share()}>
              {busy ? <Loader2 className="animate-spin" /> : <Share />}
              {t("Share…")}
            </Button>
          </div>
        </div>
      </div>

      <div className="scrollbar-visible grid w-72 shrink-0 content-start gap-4 overflow-y-auto pr-1">
        <Section title={t("Size")}>
          <Select
            value={preset}
            onValueChange={(id) => {
              const next = SIZE_PRESETS.find((p) => p.id === id);
              if (next) set({ width: next.width, height: next.height });
            }}
          >
            <SelectTrigger className="w-full" aria-label={t("Size")}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SIZE_PRESETS.map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  {t(p.label)}
                </SelectItem>
              ))}
              <SelectItem value="custom">{t("Custom size")}</SelectItem>
            </SelectContent>
          </Select>
          <div className="flex items-center gap-2 text-sm">
            <SizeInput
              label={t("Width (px)")}
              value={scene.width}
              onChange={(width) => set({ width })}
            />
            ×
            <SizeInput
              label={t("Height (px)")}
              value={scene.height}
              onChange={(height) => set({ height })}
            />
            px
          </div>
        </Section>

        <Section title={t("Background")}>
          <Select
            value={scene.background}
            onValueChange={(v) => set({ background: v as BackgroundKind })}
          >
            <SelectTrigger className="w-full" aria-label={t("Background")}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {BACKGROUNDS.map((b) => (
                <SelectItem key={b.value} value={b.value} disabled={b.value === "map" && !route}>
                  {t(b.label)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {scene.background === "color" && (
            <ColorField
              label={t("Colour")}
              value={scene.color}
              onChange={(color) => set({ color })}
            />
          )}
          {scene.background === "gradient" && (
            <>
              <ColorField
                label={t("From")}
                value={scene.gradientFrom}
                onChange={(gradientFrom) => set({ gradientFrom })}
              />
              <ColorField
                label={t("To")}
                value={scene.gradientTo}
                onChange={(gradientTo) => set({ gradientTo })}
              />
              <Range
                label={t("Direction")}
                min={0}
                max={360}
                step={5}
                value={scene.gradientAngle}
                onChange={(gradientAngle) => set({ gradientAngle })}
              />
            </>
          )}
          {scene.background === "image" && (
            <>
              <div className="grid grid-cols-4 gap-1.5">
                {photos.map((m) => (
                  <button
                    key={m.id}
                    type="button"
                    title={m.filePath}
                    className={cn(
                      "aspect-square overflow-hidden rounded-md border-2",
                      photo === m.filePath ? "border-primary" : "border-transparent",
                    )}
                    onClick={() => setImagePath(m.filePath)}
                  >
                    <img
                      src={convertFileSrc(m.filePath)}
                      alt=""
                      className="size-full object-cover"
                    />
                  </button>
                ))}
              </div>
              <Button variant="outline" size="sm" onClick={() => void chooseImage()}>
                <ImagePlus />
                {t("Choose image…")}
              </Button>
              {!photo && (
                <p className="text-xs text-muted-foreground">
                  {t("This activity has no photos; choose an image.")}
                </p>
              )}
            </>
          )}
          {scene.background === "map" && (
            <div className="flex gap-2">
              {[false, true].map((dark) => (
                <Button
                  key={String(dark)}
                  size="sm"
                  variant={scene.mapDark === dark ? "default" : "outline"}
                  onClick={() => set({ mapDark: dark })}
                >
                  {dark ? t("Dark") : t("Light")}
                </Button>
              ))}
            </div>
          )}
          {(scene.background === "image" || scene.background === "map") && (
            <Range
              label={t("Darken")}
              min={0}
              max={0.8}
              step={0.05}
              value={scene.dim}
              onChange={(dim) => set({ dim })}
            />
          )}
        </Section>

        <Section title={t("Route")}>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={scene.route.show && route !== null}
              disabled={!route}
              onCheckedChange={(checked) => setRoute({ show: checked === true })}
            />
            {route ? t("Show the route") : t("This activity has no GPS track.")}
          </label>
          {route && scene.route.show && (
            <>
              <ColorField
                label={t("Colour")}
                value={scene.route.color}
                onChange={(color) => setRoute({ color })}
              />
              <Range
                label={t("Size")}
                min={0.2}
                max={1.2}
                step={0.05}
                value={scene.route.size}
                onChange={(size) => setRoute({ size })}
              />
              <Range
                label={t("Line width")}
                min={0.3}
                max={3}
                step={0.1}
                value={scene.route.lineWidth}
                onChange={(lineWidth) => setRoute({ lineWidth })}
              />
            </>
          )}
        </Section>

        <Section title={t("Activity info")}>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={scene.info.show}
              onCheckedChange={(checked) => setInfo({ show: checked === true })}
            />
            {t("Show the facts")}
          </label>
          {scene.info.show && (
            <>
              <div className="grid grid-cols-2 gap-x-3 gap-y-2">
                {STAT_KEYS.map((key) => (
                  <label key={key} className="flex items-center gap-2 text-sm">
                    <Checkbox
                      checked={scene.info.fields.includes(key)}
                      onCheckedChange={(checked) =>
                        setInfo({
                          // Kept in the fixed order, whatever order they're ticked in.
                          fields: STAT_KEYS.filter((k) =>
                            k === key ? checked === true : scene.info.fields.includes(k),
                          ),
                        })
                      }
                    />
                    {key === "pace" && activity.viewMode === "Cycling"
                      ? t("Average speed")
                      : t(STAT_LABELS[key])}
                  </label>
                ))}
              </div>
              <ColorField
                label={t("Text colour")}
                value={scene.info.color}
                onChange={(color) => setInfo({ color })}
              />
              <Range
                label={t("Text size")}
                min={0.4}
                max={2}
                step={0.05}
                value={scene.info.size}
                onChange={(size) => setInfo({ size })}
              />
            </>
          )}
        </Section>

        <Section title={t("Grid")}>
          <label className="flex items-center gap-2 text-sm">
            <Checkbox
              checked={scene.grid.show}
              onCheckedChange={(checked) =>
                set({ grid: { ...scene.grid, show: checked === true } })
              }
            />
            {t("Show the grid and snap to it")}
          </label>
          {scene.grid.show && (
            <>
              <Select
                value={String(scene.grid.divisions)}
                onValueChange={(v) => set({ grid: { ...scene.grid, divisions: Number(v) } })}
              >
                <SelectTrigger className="w-full" aria-label={t("Grid size")}>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {GRID_DIVISIONS.map((n) => (
                    <SelectItem key={n} value={String(n)}>
                      {t("{{count}} squares across", { count: n })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                {t("Hold Option (⌥) while dragging to place freely.")}
              </p>
            </>
          )}
        </Section>

        <Button
          variant="ghost"
          size="sm"
          onClick={() =>
            setScene((current) => ({
              ...current,
              route: { ...current.route, x: DEFAULT_SCENE.route.x, y: DEFAULT_SCENE.route.y },
              info: { ...current.info, positions: DEFAULT_POSITIONS },
            }))
          }
        >
          <RotateCcw />
          {t("Reset positions")}
        </Button>
      </div>
    </div>
  );
}
