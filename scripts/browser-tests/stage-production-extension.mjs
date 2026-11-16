/** Stages the production extension with its repository-level artwork. */
import assert from "node:assert/strict";
import { copyFile, mkdir, readFile, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const SOURCE = join(ROOT, "beyond-gate-browser-extension");
const DEFAULT_OUTPUT = join(ROOT, "target", "browser-extension");
const EXTENSION_FILES = [
  "manifest.json", "launch.mjs", "flow.mjs", "login.html", "login.css", "login.mjs",
];

/** Adds every repository asset path referenced by the manifest to the output set. */
function addManifestAssets(value, assets) {
  if (typeof value === "string" && value.startsWith("assets/")) {
    assets.add(value);
  } else if (Array.isArray(value)) {
    for (const item of value) addManifestAssets(item, assets);
  } else if (value && typeof value === "object") {
    for (const item of Object.values(value)) addManifestAssets(item, assets);
  }
}

/** Adds local artwork referenced by HTML or CSS attributes and URL functions. */
function addDocumentAssets(content, assets) {
  const references = [
    ...content.matchAll(/(?:src|href)\s*=\s*["']([^"']+)["']/giu),
    ...content.matchAll(/url\(\s*["']?([^)"']+)["']?\s*\)/giu),
  ];
  for (const [, reference] of references) {
    if (reference.startsWith("assets/")) assets.add(reference);
  }
}

/** Rejects paths that could escape the repository asset directory. */
function assertSafeAssetPath(assetPath) {
  assert.ok(
    assetPath.startsWith("assets/") &&
      !assetPath.split("/").some((part) => part === "" || part === "." || part === "..") &&
      !assetPath.includes("\\"),
    `Unsafe extension asset path: ${assetPath}`,
  );
}

/** Creates a loadable extension directory without modifying the source tree. */
export async function stageProductionExtension(destination) {
  const output = resolve(destination);
  const manifest = JSON.parse(await readFile(join(SOURCE, "manifest.json"), "utf8"));
  const assets = new Set();
  addManifestAssets(manifest, assets);
  for (const file of ["login.html", "login.css"]) {
    addDocumentAssets(await readFile(join(SOURCE, file), "utf8"), assets);
  }
  for (const asset of assets) assertSafeAssetPath(asset);

  await mkdir(dirname(output), { recursive: true });
  try {
    await mkdir(output);
  } catch (error) {
    if (error.code === "EEXIST") {
      throw new Error("Extension destination already exists; choose a new directory.", { cause: error });
    }
    throw error;
  }
  try {
    for (const file of EXTENSION_FILES) {
      await copyFile(join(SOURCE, file), join(output, file));
    }
    for (const asset of assets) {
      const source = join(ROOT, ...asset.split("/"));
      const destinationPath = join(output, ...asset.split("/"));
      await mkdir(dirname(destinationPath), { recursive: true });
      await copyFile(source, destinationPath);
    }
  } catch (error) {
    await rm(output, { recursive: true, force: true });
    throw error;
  }
  return output;
}

/** Runs the documented default staging command or stages to one new output directory. */
async function main() {
  const args = process.argv.slice(2);
  assert.ok(args.length <= 1, "Usage: node stage-production-extension.mjs [new-output-directory]");
  const output = args.length === 0 ? DEFAULT_OUTPUT : resolve(args[0]);
  const staged = await stageProductionExtension(output);
  process.stdout.write(`${staged}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
