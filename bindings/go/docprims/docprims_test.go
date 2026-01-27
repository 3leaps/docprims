package docprims

import (
	"encoding/base64"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"testing"
)

func repoRoot() string {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		panic("runtime.Caller failed")
	}
	// bindings/go/docprims/docprims_test.go -> repo root (3 levels up)
	return filepath.Clean(filepath.Join(filepath.Dir(file), "..", "..", ".."))
}

func readFixture(rel string) []byte {
	b, err := os.ReadFile(filepath.Join(repoRoot(), rel))
	if err != nil {
		panic(err)
	}
	return b
}

func readB64Fixture(rel string) []byte {
	s := string(readFixture(rel))
	s = strings.ReplaceAll(s, "\n", "")
	out, err := base64.StdEncoding.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return out
}

func TestExtractBytesMarkdown(t *testing.T) {
	out, err := ExtractBytes("mem://input.md", []byte("# H\n\npara\n"), nil)
	if err != nil {
		t.Fatalf("extract failed: %v", err)
	}
	if !strings.Contains(string(out), docprimsSchemaID()) {
		t.Fatalf("expected schema_id in output")
	}
}

func TestExtractBytesDocx(t *testing.T) {
	data := readB64Fixture("testdata/fixtures/ooxml/mini.docx.b64")
	out, err := ExtractBytes("mem://upload.docx", data, nil)
	if err != nil {
		t.Fatalf("extract failed: %v", err)
	}
	if !strings.Contains(string(out), "\"kind\":\"docx\"") {
		t.Fatalf("expected docx kind in output")
	}
}

func docprimsSchemaID() string {
	return "\"schema_id\":\"https://schemas.3leaps.dev/docprims/extract/v0/docprims-extract.schema.json\""
}
