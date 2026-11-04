import type { AudioAsset, Tag } from "../lib/types";
import type { LibraryController } from "../lib/library-controller";

export function AssetMetadata({ asset, tags, disabled, controller }: {
  asset: AudioAsset; tags: Tag[]; disabled: boolean; controller: LibraryController;
}) {
  return <div className="asset-metadata">
    <button type="button" disabled={disabled} aria-pressed={asset.favorite} aria-label={`Favorite ${asset.name}`}
      onClick={() => { void controller.setFavorite(asset.id, !asset.favorite); }}>
      {asset.favorite ? "★ Favorite" : "☆ Favorite"}
    </button>
    <p>Tags: {asset.tags.length ? asset.tags.map((tag) => tag.name).join(", ") : "None"}</p>
    <details><summary>Edit tags for {asset.name}</summary>
      <fieldset disabled={disabled}><legend>Tags for {asset.name} (up to 32)</legend>
        {tags.length === 0 && <p>Create a tag above to assign it.</p>}
        {tags.map((tag) => {
          const checked = asset.tags.some((assigned) => assigned.id === tag.id);
          return <label className="tag-choice" key={tag.id}><input type="checkbox" checked={checked}
            disabled={!checked && asset.tags.length >= 32} onChange={() => {
              const ids = asset.tags.map((assigned) => assigned.id);
              void controller.setTags(asset.id, checked ? ids.filter((id) => id !== tag.id) : [...ids, tag.id]);
            }} />{tag.name}</label>;
        })}
      </fieldset>
    </details>
  </div>;
}
