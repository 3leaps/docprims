import type { DocprimsExtractV0, ExtractOptions } from "./types";

import { callJsonReturn, callJsonString, loadDocprims } from "./ffi";

function optsToJson(opts?: ExtractOptions): string {
	if (!opts) return "";
	return JSON.stringify(opts);
}

export function version(): string {
	return loadDocprims().docprimsVersion();
}

export function abiVersion(): number {
	return loadDocprims().docprimsAbiVersion();
}

export function extractFileJson(
	filePath: string,
	options?: ExtractOptions,
): string {
	return callJsonString(() =>
		loadDocprims().docprimsExtractFileJson(filePath, optsToJson(options)),
	);
}

export function extractBytesJson(
	sourceUri: string,
	data: Uint8Array,
	options?: ExtractOptions,
): string {
	return callJsonString(() =>
		loadDocprims().docprimsExtractBytesJson(
			sourceUri,
			data,
			optsToJson(options),
		),
	);
}

export function extractFile(
	filePath: string,
	options?: ExtractOptions,
): DocprimsExtractV0 {
	return callJsonReturn(() =>
		loadDocprims().docprimsExtractFileJson(filePath, optsToJson(options)),
	) as DocprimsExtractV0;
}

export function extractBytes(
	sourceUri: string,
	data: Uint8Array,
	options?: ExtractOptions,
): DocprimsExtractV0 {
	return callJsonReturn(() =>
		loadDocprims().docprimsExtractBytesJson(
			sourceUri,
			data,
			optsToJson(options),
		),
	) as DocprimsExtractV0;
}

export { DocprimsError, DocprimsErrorCode } from "./errors";
export type { DocprimsExtractV0, ExtractOptions, ExtractLimits } from "./types";
