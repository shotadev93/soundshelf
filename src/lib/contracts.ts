import type { AppError, AssetList, AssetPage, AudioAsset, LibraryInfo, PlaybackSource, SavedLibrary, ScanPhase, ScanStatus } from "./types.ts";

function invalid(): never {
  throw new Error("SoundShelf returned an invalid IPC response.");
}

function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) return invalid();
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function text(value: unknown): string {
  return typeof value === "string" ? value : invalid();
}

function nullableText(value: unknown): string | null {
  return value === null ? null : text(value);
}

function count(value: unknown): number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 && value <= 0xffffffff ? value : invalid();
}

function array(value: unknown): unknown[] {
  return Array.isArray(value) ? value : invalid();
}

export function parseError(value: unknown): AppError {
  const data = record(value);
  const code = data.code;
  if (code !== "notAuthorized" && code !== "unavailable" && code !== "invalidInput" && code !== "busy" && code !== "internalError" && code !== "databaseError" && code !== "notFound") return invalid();
  return { code, message: text(data.message) };
}

export function toAppError(value: unknown): AppError {
  try {
    return parseError(value);
  } catch {
    return { code: "unavailable", message: "The operation failed. Retry or select the directory again." };
  }
}

export function parseLibrary(value: unknown): LibraryInfo | null {
  if (value === null) return null;
  const data = record(value);
  return { id: text(data.id), displayPath: text(data.displayPath) };
}

export function parseAsset(value: unknown): AudioAsset {
  const data = record(value);
  const availability = data.availability;
  if (availability !== "available" && availability !== "inaccessible" && availability !== "missing" && availability !== "unknown") return invalid();
  const duration = data.durationSeconds;
  if (duration !== null && (typeof duration !== "number" || !Number.isFinite(duration) || duration < 0)) return invalid();
  return {
    id: text(data.id), name: text(data.name), relativeDisplayPath: text(data.relativeDisplayPath),
    durationSeconds: duration, extension: text(data.extension), availability,
  };
}

function phase(value: unknown): ScanPhase {
  switch (value) {
    case "idle": case "running": case "complete": case "partial": case "failed": return value;
    default: return invalid();
  }
}

export function parseStatus(value: unknown): ScanStatus {
  const data = record(value);
  return {
    scanId: nullableText(data.scanId), phase: phase(data.phase), visitedEntries: count(data.visitedEntries),
    discoveredAssets: count(data.discoveredAssets), warningCount: count(data.warningCount),
    warnings: array(data.warnings).map((warning) => {
      const item = record(warning);
      return { message: text(item.message), relativeDisplayPath: nullableText(item.relativeDisplayPath) };
    }),
    error: data.error === null ? null : parseError(data.error),
  };
}

export function parseAssets(value: unknown): AssetList {
  const data = record(value);
  return { scanId: nullableText(data.scanId), assets: array(data.assets).map(parseAsset) };
}

export function parsePage(value: unknown): AssetPage {
  const data = record(value);
  const page = { ...parseAssets(data), total: count(data.total), offset: count(data.offset), limit: count(data.limit) };
  if (page.limit < 1 || page.limit > 200 || page.offset > 1_000_000 || page.assets.length > page.limit || page.assets.length > page.total) return invalid();
  return page;
}

export function parseLibraries(value: unknown): SavedLibrary[] {
  return array(value).map((item) => {
    const data = record(item);
    const library = parseLibrary(data.library);
    if (library === null || typeof data.active !== "boolean") return invalid();
    return { library, active: data.active };
  });
}

export function parsePlayback(value: unknown): PlaybackSource {
  const data = record(value);
  const url = text(data.url);
  if (!/^http:\/\/127\.0\.0\.1:([1-9][0-9]{0,4})\/audio\/[a-f0-9]{32}$/.test(url)) return invalid();
  if (Number(new URL(url).port) > 65535) return invalid();
  return { url };
}
