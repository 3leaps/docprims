// Point the root package at the platform packages published with it.
//
// The committed package.json lists no platform packages, so the lock file
// stays installable with `npm ci`. Before publishing, this script writes
// optionalDependencies for exactly the platform packages in the given
// directory (one subdirectory per `napi.targets` entry), at the root version.
//
// Usage: node scripts/set-platform-packages.js <npm-packages-dir>

const fs = require("node:fs");
const path = require("node:path");

function fail(message) {
	console.error(`error: ${message}`);
	process.exit(1);
}

function main() {
	const dir = process.argv[2];
	if (!dir) fail("usage: set-platform-packages.js <npm-packages-dir>");

	const rootPath = path.resolve(__dirname, "..", "package.json");
	const root = JSON.parse(fs.readFileSync(rootPath, "utf8"));
	const targets = root.napi?.targets ?? [];
	if (root.optionalDependencies) {
		fail("package.json already lists optionalDependencies");
	}

	const names = [];
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		if (!entry.isDirectory()) fail(`unexpected entry ${entry.name}`);
		const manifest = JSON.parse(
			fs.readFileSync(path.join(dir, entry.name, "package.json"), "utf8"),
		);
		if (manifest.name !== `${root.name}-${entry.name}`) {
			fail(`${entry.name}: package name ${manifest.name}`);
		}
		if (manifest.version !== root.version) {
			fail(`${manifest.name}: version ${manifest.version} != ${root.version}`);
		}
		names.push(manifest.name);
	}
	if (names.length !== targets.length) {
		fail(`${names.length} platform packages for ${targets.length} napi targets`);
	}

	root.optionalDependencies = Object.fromEntries(
		names.sort().map((name) => [name, root.version]),
	);
	fs.writeFileSync(rootPath, `${JSON.stringify(root, null, 2)}\n`);
	console.log(`[ok] ${names.length} platform packages at ${root.version}`);
}

main();
