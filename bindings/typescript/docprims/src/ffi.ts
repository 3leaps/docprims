import fs from "node:fs";
import path from "node:path";

import { DocprimsError, DocprimsErrorCode } from "./errors";
import { loadNativeBinding } from "./native";

let cached: DocprimsLib | null = null;

export function loadDocprims(): DocprimsLib {
	if (cached) return cached;

	const packageRoot = findPackageRoot(__dirname);
	const api: DocprimsLib = loadNativeBinding(packageRoot);
	cached = api;
	return cached;
}

function findPackageRoot(startDir: string): string {
	let current = startDir;
	for (let i = 0; i < 8; i++) {
		const candidate = path.join(current, "package.json");
		if (fs.existsSync(candidate)) return current;
		const parent = path.dirname(current);
		if (parent === current) break;
		current = parent;
	}
	throw new Error("Could not locate package root (package.json not found)");
}

export type DocprimsCallJsonResult = {
	code: number;
	json?: string;
	message?: string;
};

export type DocprimsLib = {
	docprimsVersion: () => string;
	docprimsAbiVersion: () => number;
	docprimsExtractFileJson: (
		path: string,
		optionsJson: string,
	) => DocprimsCallJsonResult;
	docprimsExtractBytesJson: (
		sourceUri: string,
		data: Uint8Array,
		optionsJson: string,
	) => DocprimsCallJsonResult;
};

function raiseDocprimsError(code: number, message?: string): never {
	const codeNameSuffix = ` (code=${code})`;
	throw new DocprimsError(
		code as DocprimsErrorCode,
		message && message.length > 0 ? message : `docprims error${codeNameSuffix}`,
	);
}

export function callJsonReturn(fn: () => DocprimsCallJsonResult): unknown {
	const r = fn();
	if (r.code !== DocprimsErrorCode.Ok) {
		raiseDocprimsError(r.code, r.message);
	}
	return JSON.parse(r.json as string);
}

export function callJsonString(fn: () => DocprimsCallJsonResult): string {
	const r = fn();
	if (r.code !== DocprimsErrorCode.Ok) {
		raiseDocprimsError(r.code, r.message);
	}
	return r.json as string;
}
