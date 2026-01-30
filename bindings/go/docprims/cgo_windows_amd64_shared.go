//go:build windows && amd64 && docprims_shared

package docprims

// Windows builds use MinGW (x86_64-pc-windows-gnu target) for CGo compatibility.
// For docprims_shared, link against the import library (libdocprims_ffi.dll.a).
// The runtime DLL (docprims_ffi.dll) must be on PATH at runtime.

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib-shared/local/windows-amd64 -L${SRCDIR}/lib-shared/windows-amd64 -ldocprims_ffi -lws2_32 -luserenv -lbcrypt -lkernel32 -lntdll -ladvapi32 -liphlpapi -lpsapi
*/
import "C"
