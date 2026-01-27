//go:build linux && arm64

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib/local/linux-arm64 -L${SRCDIR}/lib/linux-arm64 -ldocprims_ffi -lm -lpthread -ldl
*/
import "C"
