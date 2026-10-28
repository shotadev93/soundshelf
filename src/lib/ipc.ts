import { invoke } from "@tauri-apps/api/core";
import { parsePage, parseLibraries, parseLibrary, parseStatus } from "./contracts.ts";
import type { AssetPage, AssetQuery, SavedLibrary, LibraryInfo, LibraryRequest, ScanStatus } from "./types.ts";

export interface LibraryApi {
  selectDirectory(): Promise<LibraryInfo | null>;
  startScan(libraryId: string): Promise<ScanStatus>;
  getStatus(libraryId: string): Promise<ScanStatus>;
  listAssets(query: AssetQuery): Promise<AssetPage>;
  listLibraries(): Promise<SavedLibrary[]>;
}

const request = (libraryId: string): { request: LibraryRequest } => ({ request: { libraryId } });

export const libraryApi: LibraryApi = {
  async selectDirectory() { return parseLibrary(await invoke<unknown>("select_directory")); },
  async startScan(id) { return parseStatus(await invoke<unknown>("scan_library", request(id))); },
  async getStatus(id) { return parseStatus(await invoke<unknown>("get_scan_status", request(id))); },
  async listAssets(query) { return parsePage(await invoke<unknown>("list_assets", { request: query })); },
  async listLibraries() { return parseLibraries(await invoke<unknown>("list_libraries")); },
};
