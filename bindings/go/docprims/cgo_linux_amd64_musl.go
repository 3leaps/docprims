//go:build linux && amd64 && musl

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib/local/linux-amd64-musl -L${SRCDIR}/lib/linux-amd64-musl -ldocprims_ffi -lm -lpthread
*/
import "C"
