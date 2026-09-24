import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
	abiVersion,
	DocprimsError,
	DocprimsErrorCode,
	extractBytes,
	extractFile,
	version,
} from "../src/index";
import { loadNativeBinding } from "../src/native";

function packageRoot(): string {
	let current = __dirname;
	for (let i = 0; i < 10; i++) {
		if (fs.existsSync(path.join(current, "package.json"))) return current;
		current = path.dirname(current);
	}
	throw new Error("package.json not found");
}

const repoRoot = path.resolve(packageRoot(), "..", "..", "..");

function fixtureBytes(rel: string): Buffer {
	const p = path.join(repoRoot, rel);
	if (!rel.endsWith(".b64")) return fs.readFileSync(p);
	return Buffer.from(fs.readFileSync(p, "utf8").replace(/\s+/g, ""), "base64");
}

function expectCode(fn: () => unknown, code: DocprimsErrorCode): void {
	assert.throws(fn, (e: unknown) => {
		assert.ok(e instanceof DocprimsError, `not a DocprimsError: ${e}`);
		assert.equal(e.code, code, `code ${e.code} (${e.message})`);
		return true;
	});
}

// The native addon must reproduce the committed v0 goldens exactly: the same
// contract the Rust CLI and FFI surfaces are held to.
const goldens: Array<[string, string, string]> = [
	["markdown-simple", "testdata/fixtures/text/simple.md", "testdata/fixtures/text/simple.md"],
	["html-simple", "testdata/fixtures/text/simple.html", "testdata/fixtures/text/simple.html"],
	["xml-simple", "testdata/fixtures/text/simple.xml", "testdata/fixtures/text/simple.xml"],
	["docx-mini", "testdata/fixtures/ooxml/mini.docx.b64", "testdata/fixtures/ooxml/mini.docx"],
	["xlsx-mini", "testdata/fixtures/ooxml/mini.xlsx.b64", "testdata/fixtures/ooxml/mini.xlsx"],
	["pptx-mini", "testdata/fixtures/ooxml/mini.pptx.b64", "testdata/fixtures/ooxml/mini.pptx"],
];

for (const [name, fixture, uri] of goldens) {
	test(`extractBytes() reproduces the ${name} v0 golden`, () => {
		const expected = JSON.parse(
			fs.readFileSync(path.join(repoRoot, "testdata", "golden", "v0", `${name}.json`), "utf8"),
		);
		const got = extractBytes(uri, fixtureBytes(fixture));
		// Goldens pin the contract, not the library version.
		got.generator.version = expected.generator.version;
		assert.deepEqual(got, expected);
	});
}

test("extractBytes() accepts a plain Uint8Array, not only Buffer", () => {
	const u8 = new TextEncoder().encode("# Title\n\nbody text\n");
	assert.ok(!Buffer.isBuffer(u8));
	assert.ok(extractBytes("mem://x.md", u8).document.text.includes("body text"));
});

test("extractBytes() honours the byteOffset of a view into a larger buffer", () => {
	const docx = fixtureBytes("testdata/fixtures/ooxml/mini.docx.b64");
	const whole = extractBytes("mem://x.docx", docx);

	// Surround the archive with junk so a binding that ignored the view's
	// offset or length would see a corrupt archive.
	const backing = Buffer.alloc(docx.length + 64, 0x5a);
	docx.copy(backing, 32);
	const view = new Uint8Array(backing.buffer, backing.byteOffset + 32, docx.length);
	assert.deepEqual(extractBytes("mem://x.docx", view), whole);
});

test("malformed and unrecognised inputs map to DataInvalid", () => {
	const docx = fixtureBytes("testdata/fixtures/ooxml/mini.docx.b64");
	expectCode(() => extractBytes("mem://x.docx", Buffer.from("not a zip")), DocprimsErrorCode.DataInvalid);
	expectCode(() => extractBytes("mem://x.docx", new Uint8Array(0)), DocprimsErrorCode.DataInvalid);
	expectCode(
		() => extractBytes("mem://x.xlsx", docx.subarray(0, docx.length >> 1)),
		DocprimsErrorCode.DataInvalid,
	);
	expectCode(() => extractBytes("mem://x.bin", Buffer.from("x")), DocprimsErrorCode.DataInvalid);
	expectCode(() => extractBytes("mem://noext", Buffer.from("x")), DocprimsErrorCode.DataInvalid);
	expectCode(
		() => extractBytes("mem://x.md", Buffer.from([0x23, 0x20, 0xff, 0xfe])),
		DocprimsErrorCode.DataInvalid,
	);
});

test("invalid options are a usage error, not silently defaulted", () => {
	expectCode(
		() =>
			extractBytes("mem://x.md", Buffer.from("# x"), {
				limits: { max_blocks: -1 },
			}),
		DocprimsErrorCode.Usage,
	);
});

test("file-path errors map to Io and ResourceLimit", () => {
	const missing = path.join(os.tmpdir(), `docprims-missing-${process.pid}.md`);
	expectCode(() => extractFile(missing), DocprimsErrorCode.Io);

	const md = path.join(repoRoot, "testdata", "fixtures", "text", "simple.md");
	expectCode(
		() => extractFile(md, { limits: { max_input_bytes: 1 } }),
		DocprimsErrorCode.ResourceLimit,
	);
});

test("version() and abiVersion() describe the loaded addon", () => {
	const pkg = JSON.parse(fs.readFileSync(path.join(packageRoot(), "package.json"), "utf8"));
	assert.equal(version(), pkg.version);
	assert.equal(abiVersion(), 1);
});

test("loader explains a missing native addon instead of crashing", (t) => {
	const empty = fs.mkdtempSync(path.join(os.tmpdir(), "docprims-noaddon-"));
	let platformPkgInstalled = false;
	try {
		loadNativeBinding(empty);
		platformPkgInstalled = true;
	} catch (e) {
		assert.ok(e instanceof Error);
		assert.match(e.message, /Failed to load docprims native addon/);
		assert.match(e.message, /@3leaps\/docprims-/);
	}
	if (platformPkgInstalled) {
		t.skip("platform package installed; absent-addon path not reachable here");
	}
});
