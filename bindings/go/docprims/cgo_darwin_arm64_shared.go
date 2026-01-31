//go:build darwin && arm64 && docprims_shared

package docprims

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo LDFLAGS: -L${SRCDIR}/lib-shared/local/darwin-arm64 -L${SRCDIR}/lib-shared/darwin-arm64 -Wl,-rpath,${SRCDIR}/lib-shared/local/darwin-arm64 -Wl,-rpath,${SRCDIR}/lib-shared/darwin-arm64 -ldocprims_ffi -framework CoreFoundation -framework Security
*/
import "C"
