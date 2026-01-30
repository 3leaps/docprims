//go:build linux && amd64 && !musl && !docprims_shared

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib/local/linux-amd64 -L${SRCDIR}/lib/linux-amd64 -ldocprims_ffi -lm -lpthread -ldl
*/
import "C"
