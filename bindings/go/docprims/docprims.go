package docprims

/*
#cgo darwin LDFLAGS: -L${SRCDIR}/../../../target/release -ldocprims_ffi
#cgo linux LDFLAGS: -L${SRCDIR}/../../../target/release -ldocprims_ffi

#include <stddef.h>
#include <stdlib.h>
#include <stdint.h>

typedef enum {
  DOCPRIMS_OK = 0,
  DOCPRIMS_INTERNAL = 1,
  DOCPRIMS_DATA_INVALID = 60,
  DOCPRIMS_USAGE = 64,
  DOCPRIMS_RESOURCE_LIMIT = 70,
  DOCPRIMS_IO = 74,
} DocprimsErrorCode;

DocprimsErrorCode docprims_extract_bytes_json(
    const uint8_t* data,
    size_t len,
    const char* source_uri,
    const char* options_json,
    char** out_json);

DocprimsErrorCode docprims_last_error_code(void);
char* docprims_last_error(void);
void docprims_clear_error(void);
void docprims_free_string(char* s);
const char* docprims_version(void);
uint32_t docprims_abi_version(void);
*/
import "C"

import (
	"encoding/json"
	"errors"
	"runtime"
	"unsafe"
)

type Limits struct {
	MaxInputBytes  uint64 `json:"max_input_bytes,omitempty"`
	MaxOutputBytes uint64 `json:"max_output_bytes,omitempty"`
	MaxBlocks      uint64 `json:"max_blocks,omitempty"`
}

type Options struct {
	Limits *Limits `json:"limits,omitempty"`
}

type Error struct {
	Code    uint32
	Message string
}

func (e *Error) Error() string {
	if e == nil {
		return "docprims: <nil>"
	}
	if e.Message != "" {
		return e.Message
	}
	return "docprims: error"
}

func Version() string {
	p := C.docprims_version()
	if p == nil {
		return ""
	}
	return C.GoString(p)
}

func ABIVersion() uint32 {
	return uint32(C.docprims_abi_version())
}

func ExtractBytes(sourceURI string, data []byte, opts *Options) ([]byte, error) {
	if sourceURI == "" {
		return nil, errors.New("sourceURI is required")
	}

	// FFI error state is thread-local.
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()

	C.docprims_clear_error()

	cSource := C.CString(sourceURI)
	defer C.free(unsafe.Pointer(cSource))

	var cOpts *C.char
	if opts != nil {
		b, err := json.Marshal(opts)
		if err != nil {
			return nil, err
		}
		cOpts = C.CString(string(b))
		defer C.free(unsafe.Pointer(cOpts))
	}

	var out *C.char

	var pData *C.uint8_t
	if len(data) != 0 {
		pData = (*C.uint8_t)(unsafe.Pointer(&data[0]))
	}

	code := C.docprims_extract_bytes_json(pData, C.size_t(len(data)), cSource, cOpts, &out)
	if code != C.DOCPRIMS_OK {
		// Prefer the TLS error string for detail.
		msg := C.docprims_last_error()
		goMsg := ""
		if msg != nil {
			goMsg = C.GoString(msg)
			C.docprims_free_string(msg)
		}
		return nil, &Error{Code: uint32(code), Message: goMsg}
	}
	if out == nil {
		return nil, &Error{Code: uint32(C.DOCPRIMS_INTERNAL), Message: "missing out_json"}
	}
	defer C.docprims_free_string(out)
	return []byte(C.GoString(out)), nil
}
