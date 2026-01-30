//go:build linux && amd64 && !musl && docprims_shared

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib-shared/local/linux-amd64 -L${SRCDIR}/lib-shared/linux-amd64 -ldocprims_ffi -lm -lpthread -ldl
*/
import "C"
