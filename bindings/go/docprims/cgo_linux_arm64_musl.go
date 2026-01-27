//go:build linux && arm64 && musl

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib/local/linux-arm64-musl -L${SRCDIR}/lib/linux-arm64-musl -ldocprims_ffi -lm -lpthread
*/
import "C"
