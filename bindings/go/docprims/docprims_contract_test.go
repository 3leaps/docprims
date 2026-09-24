package docprims

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"testing"
)

// The Go surface must reproduce the committed v0 goldens exactly: the same
// contract the Rust CLI and TypeScript surfaces are held to.
func TestExtractBytesMatchesV0Goldens(t *testing.T) {
	cases := []struct{ golden, fixture, uri string }{
		{"markdown-simple", "testdata/fixtures/text/simple.md", "testdata/fixtures/text/simple.md"},
		{"html-simple", "testdata/fixtures/text/simple.html", "testdata/fixtures/text/simple.html"},
		{"xml-simple", "testdata/fixtures/text/simple.xml", "testdata/fixtures/text/simple.xml"},
		{"docx-mini", "testdata/fixtures/ooxml/mini.docx.b64", "testdata/fixtures/ooxml/mini.docx"},
		{"xlsx-mini", "testdata/fixtures/ooxml/mini.xlsx.b64", "testdata/fixtures/ooxml/mini.xlsx"},
		{"pptx-mini", "testdata/fixtures/ooxml/mini.pptx.b64", "testdata/fixtures/ooxml/mini.pptx"},
	}
	for _, c := range cases {
		t.Run(c.golden, func(t *testing.T) {
			var data []byte
			if strings.HasSuffix(c.fixture, ".b64") {
				data = readB64Fixture(c.fixture)
			} else {
				data = readFixture(c.fixture)
			}
			out, err := ExtractBytes(c.uri, data, nil)
			if err != nil {
				t.Fatalf("ExtractBytes: %v", err)
			}

			var got, want map[string]any
			if err := json.Unmarshal(out, &got); err != nil {
				t.Fatalf("output is not JSON: %v", err)
			}
			wantBytes := readFixture(filepath.Join("testdata", "golden", "v0", c.golden+".json"))
			if err := json.Unmarshal(wantBytes, &want); err != nil {
				t.Fatal(err)
			}
			// Goldens pin the contract, not the library version.
			got["generator"].(map[string]any)["version"] = want["generator"].(map[string]any)["version"]
			if !reflect.DeepEqual(got, want) {
				t.Fatalf("output differs from golden\n got: %s", out)
			}
		})
	}
}

func errorCode(t *testing.T, err error) uint32 {
	t.Helper()
	var e *Error
	if !errors.As(err, &e) {
		t.Fatalf("expected *docprims.Error, got %T: %v", err, err)
	}
	return e.Code
}

func TestErrorCodesAcrossFailureClasses(t *testing.T) {
	docx := readB64Fixture("testdata/fixtures/ooxml/mini.docx.b64")
	cases := []struct {
		name string
		uri  string
		data []byte
		opts *Options
		code uint32
	}{
		{"not an archive", "mem://x.docx", []byte("plain text"), nil, 60},
		// Zero-length input crosses the cgo boundary as a nil pointer.
		{"empty input", "mem://x.docx", nil, nil, 60},
		{"truncated archive", "mem://x.pptx", docx[:len(docx)/2], nil, 60},
		{"unknown extension", "mem://x.bin", []byte("x"), nil, 60},
		{"missing extension", "mem://noext", []byte("x"), nil, 60},
		{"invalid utf-8 text", "mem://x.md", []byte{'#', ' ', 0xff, 0xfe}, nil, 60},
		{"input over limit", "mem://x.md", []byte("# heading"), &Options{Limits: &Limits{MaxInputBytes: 1}}, 70},
	}
	for _, c := range cases {
		t.Run(c.name, func(t *testing.T) {
			out, err := ExtractBytes(c.uri, c.data, c.opts)
			if err == nil {
				t.Fatalf("expected error, got output %s", out)
			}
			if got := errorCode(t, err); got != c.code {
				t.Fatalf("code = %d, want %d (%v)", got, c.code, err)
			}
			if err.Error() == "" {
				t.Fatal("error carries no message")
			}
		})
	}
}

// Error detail lives in thread-local FFI state. Concurrent callers must each
// see their own message, and a success must not surface a stale failure.
func TestConcurrentCallsKeepErrorsSeparate(t *testing.T) {
	const workers = 32
	var wg sync.WaitGroup
	failures := make(chan string, workers*20)
	for w := 0; w < workers; w++ {
		wg.Add(1)
		go func(w int) {
			defer wg.Done()
			for i := 0; i < 20; i++ {
				ext := fmt.Sprintf("e%dx%d", w, i)
				_, err := ExtractBytes("mem://file."+ext, []byte("x"), nil)
				if err == nil || !strings.Contains(err.Error(), ext) {
					failures <- fmt.Sprintf("worker %d iter %d: want message naming %q, got %v", w, i, ext, err)
				}
				out, err := ExtractBytes("mem://ok.md", []byte("# ok\n"), nil)
				if err != nil || !strings.Contains(string(out), `"ok`) {
					failures <- fmt.Sprintf("worker %d iter %d: success call failed: %v", w, i, err)
				}
			}
		}(w)
	}
	wg.Wait()
	close(failures)
	for f := range failures {
		t.Error(f)
	}
}

func TestVersionReporting(t *testing.T) {
	if Version() == "" {
		t.Fatal("Version() is empty")
	}
	if got := ABIVersion(); got != 1 {
		t.Fatalf("ABIVersion() = %d, want 1", got)
	}
}
