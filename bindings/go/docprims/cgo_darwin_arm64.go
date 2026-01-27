//go:build darwin && arm64

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib/local/darwin-arm64 -L${SRCDIR}/lib/darwin-arm64 -ldocprims_ffi
*/
import "C"
