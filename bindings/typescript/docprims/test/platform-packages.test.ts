import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

function packageRoot(): string {
	let current = __dirname;
	for (let i = 0; i < 10; i++) {
		if (fs.existsSync(path.join(current, "package.json"))) return current;
		current = path.dirname(current);
	}
	throw new Error("package.json not found");
}

const root = packageRoot();
const script = path.join(root, "scripts", "set-platform-packages.js");
const manifest = JSON.parse(
	fs.readFileSync(path.join(root, "package.json"), "utf8"),
);
const platforms = [
	"darwin-arm64",
	"linux-arm64-gnu",
	"linux-arm64-musl",
	"linux-x64-gnu",
	"linux-x64-musl",
	"win32-x64-msvc",
];

// A platform package directory as the prebuilds workflow stages it.
function stage(dir: string, platform: string, overrides: object = {}): void {
	fs.mkdirSync(path.join(dir, platform), { recursive: true });
	fs.writeFileSync(
		path.join(dir, platform, "package.json"),
		JSON.stringify({
			name: `${manifest.name}-${platform}`,
			version: manifest.version,
			...overrides,
		}),
	);
	fs.writeFileSync(path.join(dir, platform, `docprims.${platform}.node`), "addon");
}

function fixture(
	edit: (npmDir: string) => void = () => {},
): { npmDir: string; pkg: string } {
	const work = fs.mkdtempSync(path.join(os.tmpdir(), "docprims-platforms-"));
	const npmDir = path.join(work, "npm");
	for (const platform of platforms) stage(npmDir, platform);
	edit(npmDir);
	const pkg = path.join(work, "package.json");
	fs.copyFileSync(path.join(root, "package.json"), pkg);
	return { npmDir, pkg };
}

function run(...args: string[]) {
	return spawnSync(process.execPath, [script, ...args], { encoding: "utf8" });
}

test("committed package.json lists no platform packages", () => {
	assert.equal(manifest.optionalDependencies, undefined);
});

test("writes optionalDependencies for exactly the napi targets", () => {
	const { npmDir, pkg } = fixture();
	assert.equal(run("--check", npmDir, pkg).status, 0);
	assert.equal(JSON.parse(fs.readFileSync(pkg, "utf8")).optionalDependencies, undefined);

	const result = run(npmDir, pkg);
	assert.equal(result.status, 0, result.stderr);
	const written = JSON.parse(fs.readFileSync(pkg, "utf8")).optionalDependencies;
	assert.deepEqual(
		written,
		Object.fromEntries(
			platforms.map((p) => [`${manifest.name}-${p}`, manifest.version]),
		),
	);
	// A second run refuses to overwrite.
	assert.notEqual(run(npmDir, pkg).status, 0);
});

test("rejects platform directories that differ from napi.targets", () => {
	const cases: Array<[string, (npmDir: string) => void]> = [
		["missing platform", (d) => fs.rmSync(path.join(d, "win32-x64-msvc"), { recursive: true })],
		["extra platform", (d) => stage(d, "darwin-x64")],
		["renamed platform", (d) => fs.renameSync(path.join(d, "linux-x64-gnu"), path.join(d, "linux-x64"))],
		["stray file", (d) => fs.writeFileSync(path.join(d, "README.md"), "")],
		["wrong package name", (d) => stage(d, "darwin-arm64", { name: `${manifest.name}-other` })],
		["wrong version", (d) => stage(d, "darwin-arm64", { version: "0.0.0" })],
		["missing addon", (d) => fs.rmSync(path.join(d, "linux-arm64-musl", "docprims.linux-arm64-musl.node"))],
		["empty addon", (d) => fs.writeFileSync(path.join(d, "linux-arm64-musl", "docprims.linux-arm64-musl.node"), "")],
	];
	for (const [label, edit] of cases) {
		const { npmDir, pkg } = fixture(edit);
		for (const args of [["--check", npmDir, pkg], [npmDir, pkg]]) {
			const result = run(...args);
			assert.notEqual(result.status, 0, `${label} (${args[0]}) passed`);
			assert.match(result.stderr, /^error: /, label);
		}
		assert.equal(
			JSON.parse(fs.readFileSync(pkg, "utf8")).optionalDependencies,
			undefined,
			`${label} wrote package.json`,
		);
	}
});
