export interface AudioAsset {
  id: string;
  name: string;
  durationSeconds: number | null;
  relativeDisplayPath: string;
  extension: string;
  favorite: boolean;
  tags: Tag[];
  availability: "available" | "inaccessible" | "missing" | "unknown";
}

export interface LibraryInfo {
  id: string;
  displayPath: string;
}

export interface AppError {
  code: "notAuthorized" | "unavailable" | "invalidInput" | "busy" | "internalError" | "databaseError" | "notFound" | "duplicateTag";
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

export interface AssetQuery extends LibraryRequest {
  search: string;
  tagIds: string[];
  favoritesOnly: boolean;
  offset: number;
  limit: number;
}

export interface AssetPage extends AssetList {
  total: number;
  offset: number;
  limit: number;
}

export interface SavedLibrary {
  library: LibraryInfo;
  active: boolean;
}

export interface PlaybackRequest extends LibraryRequest {
  assetId: string;
}

export interface PlaybackSource {
  url: string;
}

export interface Tag { id: string; name: string }
export interface CreateTagRequest { name: string }
export interface RenameTagRequest { tagId: string; name: string }
export interface DeleteTagRequest { tagId: string }
export interface SetAssetTagsRequest extends PlaybackRequest { tagIds: string[] }
export interface SetAssetFavoriteRequest extends PlaybackRequest { favorite: boolean }
