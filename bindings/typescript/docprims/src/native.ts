import fs from "node:fs";
import path from "node:path";

import type { DocprimsLib } from "./ffi";

function isMusl(): boolean {
	// Best-effort detection for Node on Linux.
	// If `process.report` isn't available, default to musl (safer for container environments).
	const report = (
		process as unknown as { report?: { getReport?: () => unknown } }
	).report;
	if (!report || typeof report.getReport !== "function") return true;

	try {
		const r = report.getReport() as {
			header?: { glibcVersionRuntime?: string };
		};
		return !r?.header?.glibcVersionRuntime;
	} catch {
		return true;
	}
}

function bindingIdForRuntime(): string {
	const platform = process.platform;
	const arch = process.arch;

	if ((process as unknown as { versions?: { bun?: string } }).versions?.bun) {
		throw new Error(
			"docprims TypeScript bindings are not yet validated on Bun. " +
				"Run under Node.js or add a fallback path.",
		);
	}

	if (platform === "darwin") {
		if (arch !== "arm64") {
			throw new Error(
				`Unsupported platform for docprims: ${platform}/${arch}. ` +
					"macOS x64 is not supported by docprims.",
			);
		}
		return "darwin-arm64";
	}

	if (platform === "win32") {
		if (arch === "x64") return "win32-x64-msvc";
		throw new Error(`Unsupported platform for docprims: ${platform}/${arch}`);
	}

	if (platform === "linux") {
		const abi = isMusl() ? "musl" : "gnu";
		if (arch === "x64") return `linux-x64-${abi}`;
		if (arch === "arm64") return `linux-arm64-${abi}`;
		throw new Error(`Unsupported platform for docprims: ${platform}/${arch}`);
	}

	throw new Error(`Unsupported platform for docprims: ${platform}/${arch}`);
}

function localNodeFilename(bindingId: string): string {
	return `docprims.${bindingId}.node`;
}

export function loadNativeBinding(packageRoot: string): DocprimsLib {
	const bindingId = bindingIdForRuntime();
	const localNode = path.join(
		packageRoot,
		"dist",
		"native",
		localNodeFilename(bindingId),
	);

	// Prefer local build (git checkout / local path installs).
	if (fs.existsSync(localNode)) {
		// biome-ignore lint/suspicious/noExplicitAny: napi binding is runtime-loaded
		return require(localNode) as any;
	}

	throw new Error(
		"Failed to load docprims native addon for this platform.\n" +
			`Expected local build at: ${localNode}\n` +
			"\n" +
			"If you are installing from a git checkout or local path, build the native addon first:\n" +
			"  npm run build:native\n",
	);
}
