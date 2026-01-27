package docprims

/*
// Common C definitions for all platforms.
// Platform-specific linker flags live in cgo_*.go files.
#include <stdint.h>
#include <stdlib.h>

#include "docprims.h"

// Ensure a stable Go-visible type for Rust usize parameters.
typedef uintptr_t docprims_usize_t;
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

	code := C.docprims_extract_bytes_json(
		pData,
		C.docprims_usize_t(len(data)),
		cSource,
		cOpts,
		&out,
	)
	if code != 0 {
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
		return nil, &Error{Code: 1, Message: "missing out_json"}
	}
	defer C.docprims_free_string(out)
	return []byte(C.GoString(out)), nil
}
