import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
	DocprimsError,
	DocprimsErrorCode,
	extractBytes,
	extractFile,
} from "../src/index";

function findPackageRoot(startDir: string): string {
	let current = startDir;
	for (let i = 0; i < 10; i++) {
		const candidate = path.join(current, "package.json");
		if (fs.existsSync(candidate)) return current;
		const parent = path.dirname(current);
		if (parent === current) break;
		current = parent;
	}
	throw new Error("Could not locate package root (package.json not found)");
}

function repoRoot(): string {
	const pkgRoot = findPackageRoot(__dirname);
	return path.resolve(pkgRoot, "..", "..", "..");
}

test("extractFile() extracts markdown fixture", () => {
	const p = path.join(repoRoot(), "testdata", "fixtures", "text", "simple.md");
	const r = extractFile(p);

	assert.ok(r.schema_id);
	assert.ok(r.schema_version);
	assert.equal(r.source.format.kind, "markdown");
	assert.ok(r.document.text.includes("para"));
});

test("extractBytes() extracts markdown from bytes", () => {
	const r = extractBytes("mem://input.md", Buffer.from("# H\n\npara\n"));
	assert.equal(r.source.uri, "mem://input.md");
	assert.equal(r.source.format.kind, "markdown");
	assert.ok(r.document.text.includes("para"));
});

test("extractBytes() enforces max_input_bytes", () => {
	assert.throws(
		() =>
			extractBytes("mem://input.md", Buffer.from("# H\n"), {
				limits: { max_input_bytes: 1 },
			}),
		(e: unknown) =>
			e instanceof DocprimsError && e.code === DocprimsErrorCode.ResourceLimit,
	);
});

test("extractFile() extracts OOXML fixture (base64 mini docx)", () => {
	const b64Path = path.join(
		repoRoot(),
		"testdata",
		"fixtures",
		"ooxml",
		"mini.docx.b64",
	);
	const b64 = fs.readFileSync(b64Path, "utf8");
	const compact = b64
		.split("\n")
		.map((l) => l.trim())
		.filter((l) => l.length > 0)
		.join("");
	const bytes = Buffer.from(compact, "base64");

	const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "docprims-ts-"));
	const docx = path.join(tmp, "mini.docx");
	fs.writeFileSync(docx, bytes);

	const r = extractFile(docx);
	assert.equal(r.source.format.kind, "docx");
});
