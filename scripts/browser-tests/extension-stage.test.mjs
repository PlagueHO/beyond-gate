/** Offline checks for the unpacked production extension staging command. */
import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { copyFile, mkdir, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { stageProductionExtension } from "./stage-production-extension.mjs";

const ROOT = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const SOURCE = join(ROOT, "beyond-gate-browser-extension");
const EXTENSION_FILES = [
  "manifest.json", "launch.mjs", "flow.mjs", "login.html", "login.css", "login.mjs",
];

/** Adds every repository asset path referenced by the manifest. */
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

/** Lists staged files by their extension-relative paths. */
async function listFiles(directory, prefix = "") {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = prefix ? `${prefix}/${entry.name}` : entry.name;
    if (entry.isDirectory()) files.push(...await listFiles(join(directory, entry.name), path));
    else files.push(path);
  }
  return files;
}

/** Checks that staging includes exactly the files required by the production manifest and page. */
async function verifyStagedExtension() {
  const directory = await createTestDirectory();
  const output = join(directory, "extension");
  try {
    assert.equal(await stageProductionExtension(output), output);
    const manifest = JSON.parse(await readFile(join(output, "manifest.json"), "utf8"));
    const assets = new Set();
    addManifestAssets(manifest, assets);
    for (const file of ["login.html", "login.css"]) {
      addDocumentAssets(await readFile(join(output, file), "utf8"), assets);
    }
    const expected = new Set([...EXTENSION_FILES, ...assets]);
    assert.deepEqual(new Set(await listFiles(output)), expected);
    for (const file of expected) {
      const staged = await readFile(join(output, ...file.split("/")));
      assert.ok(staged.length > 0, `${file} must not be empty`);
      const source = file.startsWith("assets/") ? ROOT : SOURCE;
      assert.deepEqual(staged, await readFile(join(source, ...file.split("/"))));
    }
    assert.equal((await readdir(output)).includes("tests"), false);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

/** Creates an exclusively owned test directory within the ignored build output. */
async function createTestDirectory() {
  const parent = join(ROOT, "target");
  await mkdir(parent, { recursive: true });
  return mkdtemp(join(parent, "browser-extension-stage-"));
}

test("staging produces a complete unpacked extension without tests or duplicated artwork", verifyStagedExtension);

test("the helper and no-argument command preserve an existing destination", async () => {
  const directory = await createTestDirectory();
  try {
    const scriptDirectory = join(directory, "scripts", "browser-tests");
    const source = join(directory, "beyond-gate-browser-extension");
    const output = join(directory, "target", "browser-extension");
    await mkdir(scriptDirectory, { recursive: true });
    await mkdir(source);
    await mkdir(join(output, "user-data"), { recursive: true });
    const sentinel = join(output, "user-data", "saved.txt");
    await writeFile(sentinel, "Preserve this user content.\n");
    const script = join(scriptDirectory, "stage-production-extension.mjs");
    await copyFile(new URL("./stage-production-extension.mjs", import.meta.url), script);
    for (const file of ["manifest.json", "login.html", "login.css"]) {
      await copyFile(join(SOURCE, file), join(source, file));
    }
    await assert.rejects(stageProductionExtension(output), /destination already exists/);
    await assert.rejects(promisify(execFile)(process.execPath, [script], { cwd: directory }), (error) => {
      assert.equal(error.code, 1);
      assert.equal(error.stdout, "");
      assert.match(error.stderr, /destination already exists/);
      return true;
    });
    assert.equal(await readFile(sentinel, "utf8"), "Preserve this user content.\n");
    assert.deepEqual(await listFiles(output), ["user-data/saved.txt"]);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
