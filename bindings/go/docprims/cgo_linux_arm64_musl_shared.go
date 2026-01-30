//go:build linux && arm64 && musl && docprims_shared

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib-shared/local/linux-arm64-musl -L${SRCDIR}/lib-shared/linux-arm64-musl -ldocprims_ffi -lm -lpthread
*/
import "C"
