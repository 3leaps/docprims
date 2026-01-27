//go:build windows && amd64

package docprims

// Windows builds use MinGW (x86_64-pc-windows-gnu target) for CGo compatibility.
// The FFI library is libdocprims_ffi.a (not .lib) to work with MinGW linker.

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib/local/windows-amd64 -L${SRCDIR}/lib/windows-amd64 -ldocprims_ffi -lws2_32 -luserenv -lbcrypt -lkernel32 -lntdll -ladvapi32 -liphlpapi -lpsapi
*/
import "C"
