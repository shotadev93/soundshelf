export interface AudioAsset {
  id: string;
  name: string;
  durationSeconds: number | null;
  relativeDisplayPath: string;
  extension: string;
  availability: "available" | "inaccessible";
}

export interface LibraryInfo {
  id: string;
  displayPath: string;
}

export interface AppError {
  code: "notAuthorized" | "unavailable" | "invalidInput" | "busy" | "internalError";
  message: string;
}

export interface LibraryRequest {
  libraryId: string;
}

export type ScanPhase = "idle" | "running" | "complete" | "partial" | "failed";

export interface ScanStatus {
  scanId: string | null;
  phase: ScanPhase;
  visitedEntries: number;
  discoveredAssets: number;
  warningCount: number;
  warnings: { message: string; relativeDisplayPath: string | null }[];
  error: AppError | null;
}

export interface AssetList {
  scanId: string | null;
  assets: AudioAsset[];
}
