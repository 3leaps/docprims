//go:build linux && amd64 && musl && docprims_shared

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib-shared/local/linux-amd64-musl -L${SRCDIR}/lib-shared/linux-amd64-musl -ldocprims_ffi -lm -lpthread
*/
import "C"
