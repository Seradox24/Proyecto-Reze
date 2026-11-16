import fs from 'node:fs';
import path from 'node:path';
import zlib from 'node:zlib';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const outRoot = path.join(root, 'ui', 'src', 'assets', 'themes');

const THEMES = {
  classic_cat: { stroke: [193, 232, 179, 255], blush: null, stars: false },
  window_cat: { stroke: [243, 181, 158, 255], blush: null, stars: false },
  cat_notebook: { stroke: [114, 65, 160, 255], blush: [255, 170, 190, 220], stars: false },
  cat_constellation: { stroke: [242, 205, 138, 255], blush: null, stars: true },
  mochi_cat: { stroke: [194, 38, 94, 255], blush: [255, 157, 184, 220], stars: false },
};

const COLS = 8;
const ROWS = 6;
const CELL = 48;
const WIDTH = COLS * CELL;
const HEIGHT = ROWS * CELL;

const SEQUENCES = {
  idle: { row: 0, frames: 4, fps: 4, loop: false },
  playing: { row: 1, frames: 8, fps: 8, loop: true },
  paused: { row: 2, frames: 6, fps: 4, loop: false },
  greet: { row: 3, frames: 6, fps: 8, loop: false },
  searching: { row: 4, frames: 6, fps: 6, loop: false },
  static: { row: 5, frames: 1, fps: 1, loop: false },
};

function crcTable() {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
}
const CRC = crcTable();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC[(c ^ buf[i]) & 255] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeBuf = Buffer.from(type);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])));
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

function encodePng(rgba, width, height) {
  const stride = width * 4;
  const raw = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    const dest = y * (stride + 1);
    raw[dest] = 0;
    rgba.copy(raw, dest + 1, y * stride, (y + 1) * stride);
  }
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 6 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

function px(buf, x, y, rgba) {
  if (x < 0 || y < 0 || x >= WIDTH || y >= HEIGHT) return;
  const i = (y * WIDTH + x) * 4;
  buf[i] = rgba[0];
  buf[i + 1] = rgba[1];
  buf[i + 2] = rgba[2];
  buf[i + 3] = rgba[3];
}

function disc(buf, cx, cy, r, rgba) {
  const rr = r * r;
  for (let y = -r; y <= r; y++) {
    for (let x = -r; x <= r; x++) {
      if (x * x + y * y <= rr) px(buf, cx + x, cy + y, rgba);
    }
  }
}

function ring(buf, cx, cy, r, rgba) {
  const outer = r * r;
  const inner = (r - 2) * (r - 2);
  for (let y = -r; y <= r; y++) {
    for (let x = -r; x <= r; x++) {
      const d = x * x + y * y;
      if (d <= outer && d >= inner) px(buf, cx + x, cy + y, rgba);
    }
  }
}

function plot(buf, points, rgba) {
  for (const [x, y] of points) px(buf, x, y, rgba);
}

function drawCat(buf, ox, oy, theme, pose) {
  const s = theme.stroke;
  const ear = pose.ear ?? 0;
  const headY = oy + 18 + (pose.headY ?? 0);
  const bodyY = oy + 31 + (pose.curl ?? 0);
  const tail = pose.tail ?? 0;
  const paw = pose.paw ?? 0;

  // ears
  plot(
    buf,
    [
      [ox + 15, headY - 1],
      [ox + 13, headY - 6 + ear],
      [ox + 17, headY - 2],
      [ox + 33, headY - 1],
      [ox + 35, headY - 6 - ear],
      [ox + 31, headY - 2],
      [ox + 14, headY - 4 + ear],
      [ox + 34, headY - 4 - ear],
    ],
    s,
  );

  ring(buf, ox + 24, headY, 8, s);
  ring(buf, ox + 24, bodyY, 10, s);

  if ((pose.eye ?? 1) > 0.3) {
    disc(buf, ox + 20, headY, 1, s);
    disc(buf, ox + 28, headY, 1, s);
  } else {
    px(buf, ox + 19, headY, s);
    px(buf, ox + 20, headY, s);
    px(buf, ox + 21, headY, s);
    px(buf, ox + 27, headY, s);
    px(buf, ox + 28, headY, s);
    px(buf, ox + 29, headY, s);
  }

  if (theme.blush) {
    disc(buf, ox + 17, headY + 3, 1, theme.blush);
    disc(buf, ox + 31, headY + 3, 1, theme.blush);
  }

  // tail
  for (let i = 0; i < 8; i++) {
    px(buf, ox + 34 + i, bodyY + Math.round(Math.sin((i + tail) / 2) * 3), s);
  }

  if (paw > 0) {
    for (let i = 0; i <= paw; i++) px(buf, ox + 14 - i, bodyY - 2 - i, s);
    disc(buf, ox + 14 - paw, bodyY - 2 - paw, 2, s);
  }

  if (theme.stars) {
    disc(buf, ox + 8, oy + 10, 1, s);
    disc(buf, ox + 40, oy + 12, 1, s);
  }

  if (pose.notes) {
    px(buf, ox + 36, oy + 8, s);
    px(buf, ox + 37, oy + 7, s);
    px(buf, ox + 39, oy + 10, s);
  }
}

function posesFor(kind, frame) {
  switch (kind) {
    case 'idle':
      return { ear: frame % 2 ? 1 : -1, eye: frame === 2 ? 0 : 1 };
    case 'playing':
      return { tail: frame, headY: frame % 2, ear: (frame % 3) - 1, notes: true };
    case 'paused':
      return { curl: Math.min(frame, 4), eye: frame > 2 ? 0 : 1, tail: -frame };
    case 'greet':
      return { paw: Math.min(frame + 1, 4), ear: 1, headY: -1 };
    case 'searching':
      return { ear: frame > 2 ? 2 : -2, headY: 0 };
    default:
      return {};
  }
}

function writeTheme(id, theme) {
  const dir = path.join(outRoot, id);
  fs.mkdirSync(dir, { recursive: true });
  const rgba = Buffer.alloc(WIDTH * HEIGHT * 4, 0);

  for (const [kind, seq] of Object.entries(SEQUENCES)) {
    for (let f = 0; f < seq.frames; f++) {
      drawCat(rgba, f * CELL, seq.row * CELL, theme, posesFor(kind, f));
    }
  }

  fs.writeFileSync(path.join(dir, 'sprites.png'), encodePng(rgba, WIDTH, HEIGHT));

  const manifest = {
    version: 1,
    theme_id: id,
    frame_size: CELL,
    columns: COLS,
    rows: ROWS,
    sheet: 'sprites.png',
    sequences: Object.fromEntries(
      Object.entries(SEQUENCES).map(([key, seq]) => [
        key,
        {
          row: seq.row,
          frames: seq.frames,
          fps: seq.fps,
          loop: seq.loop,
          static_frame: key === 'paused' ? seq.frames - 1 : 0,
        },
      ]),
    ),
    posters: {
      idle: { row: 5, frame: 0 },
      playing: { row: 1, frame: 0 },
      paused: { row: 2, frame: 5 },
      searching: { row: 4, frame: 0 },
    },
  };
  fs.writeFileSync(path.join(dir, 'sprites.json'), `${JSON.stringify(manifest, null, 2)}\n`);

  const hex = `#${theme.stroke
    .slice(0, 3)
    .map((n) => n.toString(16).padStart(2, '0'))
    .join('')}`;
  fs.writeFileSync(
    path.join(dir, 'mascot.svg'),
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48" fill="none" stroke="${hex}" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">
  <path d="M16 16l-4-8 6 5M32 16l4-8-6 5"/>
  <circle cx="24" cy="20" r="8"/>
  <circle cx="24" cy="32" r="10"/>
  <path d="M33 30c4-2 8 1 9 5"/>
  <circle cx="20" cy="19" r="1.2" fill="${hex}" stroke="none"/>
  <circle cx="28" cy="19" r="1.2" fill="${hex}" stroke="none"/>
</svg>
`,
  );
  console.log('wrote', id);
}

for (const [id, theme] of Object.entries(THEMES)) writeTheme(id, theme);
console.log('done');
