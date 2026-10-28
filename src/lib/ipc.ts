import { invoke } from "@tauri-apps/api/core";
import { parseAssets, parseLibrary, parseStatus } from "./contracts.ts";
import type { AssetList, LibraryInfo, LibraryRequest, ScanStatus } from "./types.ts";

export interface LibraryApi {
  selectDirectory(): Promise<LibraryInfo | null>;
  startScan(libraryId: string): Promise<ScanStatus>;
  getStatus(libraryId: string): Promise<ScanStatus>;
  listAssets(libraryId: string): Promise<AssetList>;
}

const request = (libraryId: string): { request: LibraryRequest } => ({ request: { libraryId } });

export const libraryApi: LibraryApi = {
  async selectDirectory() { return parseLibrary(await invoke<unknown>("select_directory")); },
  async startScan(id) { return parseStatus(await invoke<unknown>("scan_library", request(id))); },
  async getStatus(id) { return parseStatus(await invoke<unknown>("get_scan_status", request(id))); },
  async listAssets(id) { return parseAssets(await invoke<unknown>("list_assets", request(id))); },
};
