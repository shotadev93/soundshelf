// Bundle the actual controller for the native WebKit regression, without a server
// or generated source checked into Git. Uses the existing Vite dependency.
import { build } from "vite";
import { fileURLToPath } from "node:url";

const bundle = await build({
  configFile: false,
  logLevel: "silent",
  build: {
    write: false,
    minify: false,
    lib: {
      entry: fileURLToPath(new URL("../src/lib/playback-controller.ts", import.meta.url)),
      formats: ["es"],
    },
  },
});
const bundles = Array.isArray(bundle) ? bundle : [bundle];
const entries = bundles.flatMap(result => "output" in result
  ? result.output.filter(output => output.type === "chunk" && output.isEntry) : []);
if (entries.length !== 1 || entries[0].type !== "chunk") throw new Error("Expected one controller bundle");
process.stdout.write(entries[0].code);
