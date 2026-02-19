/**
 * Generate minimal test fixture files.
 *
 * Usage:  node generate-fixtures.mjs
 *
 * - images/sample.png  — 4x4 red PNG (~100 bytes)
 * - videos/sample.mp4  — requires ffmpeg: 1-second black video
 * - music/sample.mp3   — requires ffmpeg: 1-second silent audio
 *
 * The PNG is generated purely in Node (no dependencies).
 * Video and audio require ffmpeg on PATH.
 */
import { writeFileSync, existsSync } from "node:fs";
import { execSync } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { deflateSync } from "node:zlib";

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── Minimal PNG generator (4x4 solid red) ──

function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    c = (c >>> 8) ^ crc32Table[(c ^ buf[i]) & 0xff];
  }
  return (c ^ 0xffffffff) >>> 0;
}
const crc32Table = new Uint32Array(256);
for (let n = 0; n < 256; n++) {
  let c = n;
  for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  crc32Table[n] = c >>> 0;
}

function pngChunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const typeAndData = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(typeAndData));
  return Buffer.concat([len, typeAndData, crc]);
}

function createMinimalPng(width, height, r, g, b) {
  const sig = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);

  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth
  ihdr[9] = 2; // color type: RGB
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const rawData = [];
  for (let y = 0; y < height; y++) {
    rawData.push(0); // filter: none
    for (let x = 0; x < width; x++) rawData.push(r, g, b);
  }
  const compressed = deflateSync(Buffer.from(rawData));

  const iend = Buffer.alloc(0);

  return Buffer.concat([
    sig,
    pngChunk("IHDR", ihdr),
    pngChunk("IDAT", compressed),
    pngChunk("IEND", iend),
  ]);
}

// ── Generate PNG ──
const pngPath = join(__dirname, "images", "sample.png");
if (!existsSync(pngPath)) {
  writeFileSync(pngPath, createMinimalPng(4, 4, 255, 0, 0));
  console.log("Created:", pngPath);
} else {
  console.log("Exists:", pngPath);
}

// ── Generate video via ffmpeg ──
const mp4Path = join(__dirname, "videos", "sample.mp4");
if (!existsSync(mp4Path)) {
  try {
    execSync(
      `ffmpeg -y -f lavfi -i color=black:s=320x240:d=1 -c:v libx264 -pix_fmt yuv420p "${mp4Path}"`,
      { stdio: "pipe" }
    );
    console.log("Created:", mp4Path);
  } catch {
    console.warn("ffmpeg not found — skipping video fixture. Install ffmpeg to generate sample.mp4");
  }
} else {
  console.log("Exists:", mp4Path);
}

// ── Generate audio via ffmpeg ──
const mp3Path = join(__dirname, "music", "sample.mp3");
if (!existsSync(mp3Path)) {
  try {
    execSync(
      `ffmpeg -y -f lavfi -i anullsrc=r=44100:cl=mono -t 1 -q:a 9 "${mp3Path}"`,
      { stdio: "pipe" }
    );
    console.log("Created:", mp3Path);
  } catch {
    console.warn("ffmpeg not found — skipping audio fixture. Install ffmpeg to generate sample.mp3");
  }
} else {
  console.log("Exists:", mp3Path);
}

console.log("Done.");
