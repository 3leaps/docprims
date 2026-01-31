//go:build linux && arm64 && !musl && docprims_shared

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib-shared/local/linux-arm64 -L${SRCDIR}/lib-shared/linux-arm64 -Wl,-rpath,${SRCDIR}/lib-shared/local/linux-arm64 -Wl,-rpath,${SRCDIR}/lib-shared/linux-arm64 -ldocprims_ffi -lm -lpthread -ldl
*/
import "C"
