import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const css = readFileSync(new URL("../App.css", import.meta.url), "utf8");
// Check the authored palettes without adding a browser dependency. Native popup
// rendering and keyboard highlight behavior still need Linux WebView checks.
const palettes = [...css.matchAll(/\.saved-libraries\s*\{([^{}]*)\}/g)].map((match) =>
  Object.fromEntries([...match[1].matchAll(/--library-([\w-]+):\s*(#[\da-f]{6})\s*;/gi)]
    .map((entry) => [entry[1], entry[2]])),
);

function luminance(hex) {
  assert.match(hex, /^#[\da-f]{6}$/i);
  const channels = [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16) / 255)
    .map((channel) => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4);
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}

function contrast(first, second) {
  const values = [luminance(first), luminance(second)];
  return (Math.max(...values) + 0.05) / (Math.min(...values) + 0.05);
}

for (const [index, theme, canvas] of [[0, "light", "#f5f6f8"], [1, "dark", "#171b22"]]) {
  test(`saved-library ${theme} theme has readable text and distinct interaction colors`, () => {
    assert.equal(palettes.length, 2);
    const palette = palettes[index];
    for (const [foreground, background] of [
      ["text", "surface"], ["text", "hover"],
      ["selected-text", "selected"], ["selected-text", "selected-hover"],
    ]) {
      const ratio = contrast(palette[foreground], palette[background]);
      assert.ok(ratio >= 4.5, `${theme} ${background}: ${ratio.toFixed(2)}:1 is below AA text contrast`);
    }
    assert.equal(new Set([palette.surface, palette.hover, palette.selected, palette["selected-hover"]]).size, 4);
    assert.ok(contrast(palette.focus, canvas) >= 3, "focus ring must stand out from the page");
    assert.ok(contrast(palette.border, palette.surface) >= 3, "control boundary must remain visible");
  });
}
