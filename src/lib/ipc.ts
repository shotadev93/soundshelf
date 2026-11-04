import { invoke } from "@tauri-apps/api/core";
import { parseTag, parseTags, parsePlayback, parsePage, parseLibraries, parseLibrary, parseStatus } from "./contracts.ts";
import type { AssetPage, AssetQuery, SavedLibrary, LibraryInfo, LibraryRequest, PlaybackRequest, PlaybackSource, ScanStatus, Tag, CreateTagRequest, RenameTagRequest, DeleteTagRequest, SetAssetTagsRequest, SetAssetFavoriteRequest } from "./types.ts";

export interface LibraryApi {
  listTags(): Promise<Tag[]>;
  createTag(request: CreateTagRequest): Promise<Tag>;
  renameTag(request: RenameTagRequest): Promise<Tag>;
  deleteTag(request: DeleteTagRequest): Promise<void>;
  setAssetTags(request: SetAssetTagsRequest): Promise<void>;
  setAssetFavorite(request: SetAssetFavoriteRequest): Promise<void>;
  selectDirectory(): Promise<LibraryInfo | null>;
  startScan(libraryId: string): Promise<ScanStatus>;
  getStatus(libraryId: string): Promise<ScanStatus>;
  listAssets(query: AssetQuery): Promise<AssetPage>;
  listLibraries(): Promise<SavedLibrary[]>;
}

const request = (libraryId: string): { request: LibraryRequest } => ({ request: { libraryId } });

export const libraryApi: LibraryApi = {
  async listTags() { return parseTags(await invoke<unknown>("list_tags")); },
  async createTag(request) { return parseTag(await invoke<unknown>("create_tag", { request })); },
  async renameTag(request) { return parseTag(await invoke<unknown>("rename_tag", { request })); },
  async deleteTag(request) { await invoke<void>("delete_tag", { request }); },
  async setAssetTags(request) { await invoke<void>("set_asset_tags", { request }); },
  async setAssetFavorite(request) { await invoke<void>("set_asset_favorite", { request }); },
  async selectDirectory() { return parseLibrary(await invoke<unknown>("select_directory")); },
  async startScan(id) { return parseStatus(await invoke<unknown>("scan_library", request(id))); },
  async getStatus(id) { return parseStatus(await invoke<unknown>("get_scan_status", request(id))); },
  async listAssets(query) { return parsePage(await invoke<unknown>("list_assets", { request: query })); },
  async listLibraries() { return parseLibraries(await invoke<unknown>("list_libraries")); },
};

export async function preparePlayback(request: PlaybackRequest): Promise<PlaybackSource> {
  return parsePlayback(await invoke<unknown>("prepare_playback", { request }));
}
