import {
  routeTransform,
  tilesFor,
  type InfoItem,
  type Route,
  type ElementId,
  type Scene,
  type Tile,
} from "./share-scene";

const FONT = '"Geist Variable", sans-serif';
const MAP_CREDIT = "Tiles © Esri — Esri, HERE, Garmin, © OpenStreetMap contributors";

export type Box = { left: number; top: number; width: number; height: number };

/** Images the scene needs, looked up by key; `undefined` while still loading. */
export type Images = {
  tile: (tile: Tile, labels: boolean) => ImageBitmap | undefined;
  background: ImageBitmap | undefined;
};

/** Where the route and each fact ended up, for selecting and dragging them in the preview. */
export type Layout = Partial<Record<ElementId, Box>>;

function gradient(ctx: CanvasRenderingContext2D, scene: Scene) {
  const { width, height, gradientAngle } = scene;
  const angle = (gradientAngle * Math.PI) / 180;
  const [dx, dy] = [Math.sin(angle), -Math.cos(angle)];
  // As CSS: the line runs through the centre and just reaches the far corners.
  const half = (Math.abs(width * dx) + Math.abs(height * dy)) / 2;
  const fill = ctx.createLinearGradient(
    width / 2 - dx * half,
    height / 2 - dy * half,
    width / 2 + dx * half,
    height / 2 + dy * half,
  );
  fill.addColorStop(0, scene.gradientFrom);
  fill.addColorStop(1, scene.gradientTo);
  return fill;
}

function cover(ctx: CanvasRenderingContext2D, image: ImageBitmap, width: number, height: number) {
  const scale = Math.max(width / image.width, height / image.height);
  const [w, h] = [image.width * scale, image.height * scale];
  ctx.drawImage(image, (width - w) / 2, (height - h) / 2, w, h);
}

function background(
  ctx: CanvasRenderingContext2D,
  scene: Scene,
  route: Route | null,
  images: Images,
) {
  const { width, height } = scene;
  switch (scene.background) {
    case "transparent":
      return;
    case "color":
      ctx.fillStyle = scene.color;
      ctx.fillRect(0, 0, width, height);
      return;
    case "gradient":
      ctx.fillStyle = gradient(ctx, scene);
      ctx.fillRect(0, 0, width, height);
      return;
    case "image":
      ctx.fillStyle = "#1f1f1f";
      ctx.fillRect(0, 0, width, height);
      if (images.background) cover(ctx, images.background, width, height);
      break;
    case "map": {
      ctx.fillStyle = scene.mapDark ? "#1e1e1e" : "#e8e8e8";
      ctx.fillRect(0, 0, width, height);
      if (route) {
        const tiles = tilesFor(routeTransform(route, scene), width, height);
        for (const labels of [false, true]) {
          for (const tile of tiles) {
            const image = images.tile(tile, labels);
            // A hair larger, so neighbouring tiles leave no seams.
            if (image) ctx.drawImage(image, tile.left, tile.top, tile.side + 0.5, tile.side + 0.5);
          }
        }
      }
      break;
    }
  }
  if (scene.dim > 0) {
    ctx.fillStyle = `rgba(0, 0, 0, ${scene.dim})`;
    ctx.fillRect(0, 0, width, height);
  }
}

function drawRoute(ctx: CanvasRenderingContext2D, scene: Scene, route: Route): Box {
  const { scale, offsetX, offsetY } = routeTransform(route, scene);
  const base = Math.min(scene.width, scene.height);
  ctx.save();
  ctx.lineWidth = Math.max(1, base * 0.008 * scene.route.lineWidth);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.strokeStyle = scene.route.color;
  ctx.shadowColor = "rgba(0, 0, 0, 0.35)";
  ctx.shadowBlur = ctx.lineWidth * 1.5;
  ctx.beginPath();
  route.points.forEach(([x, y], i) => {
    const [cx, cy] = [x * scale + offsetX, y * scale + offsetY];
    if (i === 0) ctx.moveTo(cx, cy);
    else ctx.lineTo(cx, cy);
  });
  ctx.stroke();
  ctx.restore();
  const [x0, y0, x1, y1] = route.box;
  return {
    left: x0 * scale + offsetX,
    top: y0 * scale + offsetY,
    width: (x1 - x0) * scale,
    height: (y1 - y0) * scale,
  };
}

/** One fact centred on its position: the title or date as a line, a figure with its label
 * below. */
function drawItem(ctx: CanvasRenderingContext2D, scene: Scene, item: InfoItem): Box {
  const { info } = scene;
  const unit = (Math.min(scene.width, scene.height) / 16) * info.size;
  const { x, y } = info.positions[item.key];
  const lines =
    item.key === "title"
      ? [{ font: `700 ${unit * 0.9}px ${FONT}`, text: item.value, height: unit * 1.05, alpha: 1 }]
      : item.key === "date"
        ? [
            {
              font: `400 ${unit * 0.45}px ${FONT}`,
              text: item.value,
              height: unit * 0.6,
              alpha: 0.8,
            },
          ]
        : [
            { font: `600 ${unit}px ${FONT}`, text: item.value, height: unit * 1.1, alpha: 1 },
            {
              font: `400 ${unit * 0.42}px ${FONT}`,
              text: (item.label ?? "").toUpperCase(),
              height: unit * 0.55,
              alpha: 0.75,
            },
          ];
  const width = Math.max(
    ...lines.map((line) => {
      ctx.font = line.font;
      return ctx.measureText(line.text).width;
    }),
  );
  const height = lines.reduce((sum, line) => sum + line.height, 0);
  const centreX = x * scene.width;
  let top = y * scene.height - height / 2;
  const box = { left: centreX - width / 2, top, width, height };
  ctx.save();
  ctx.fillStyle = info.color;
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  ctx.shadowColor = "rgba(0, 0, 0, 0.35)";
  ctx.shadowBlur = unit * 0.15;
  for (const line of lines) {
    ctx.font = line.font;
    ctx.globalAlpha = line.alpha;
    ctx.fillText(line.text, centreX, top);
    top += line.height;
  }
  ctx.restore();
  return box;
}

function credit(ctx: CanvasRenderingContext2D, scene: Scene) {
  const size = Math.max(10, Math.min(scene.width, scene.height) / 90);
  ctx.save();
  ctx.font = `400 ${size}px ${FONT}`;
  ctx.textAlign = "right";
  ctx.textBaseline = "bottom";
  ctx.fillStyle = scene.mapDark ? "rgba(255, 255, 255, 0.7)" : "rgba(0, 0, 0, 0.6)";
  ctx.fillText(MAP_CREDIT, scene.width - size, scene.height - size * 0.8);
  ctx.restore();
}

/** Draws the whole image onto `ctx` (sized `scene.width × scene.height`). */
export function drawShareImage(
  ctx: CanvasRenderingContext2D,
  scene: Scene,
  route: Route | null,
  items: InfoItem[],
  images: Images,
): Layout {
  ctx.clearRect(0, 0, scene.width, scene.height);
  background(ctx, scene, route, images);
  const layout: Layout = {};
  if (route && scene.route.show) layout.route = drawRoute(ctx, scene, route);
  if (scene.info.show) {
    for (const item of items) layout[item.key] = drawItem(ctx, scene, item);
  }
  // The tile terms ask for the credit wherever the map is shown.
  if (scene.background === "map" && route) credit(ctx, scene);
  return layout;
}
