#ifndef DOCPRIMS_H
#define DOCPRIMS_H

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

/**
 * Error codes returned by docprims FFI functions.
 *
 * These are intentionally coarse; callers should read `docprims_last_error()`
 * for a human-readable message.
 */
typedef enum DocprimsErrorCode {
    /**
     * Success.
     */
    Ok = 0,
    /**
     * Invalid arguments / caller usage error.
     */
    Usage = 64,
    /**
     * Input is malformed or unsupported.
     */
    DataInvalid = 60,
    /**
     * Resource limit exceeded.
     */
    ResourceLimit = 70,
    /**
     * I/O failure reading input.
     */
    Io = 74,
    /**
     * Internal failure.
     */
    Internal = 1,
} DocprimsErrorCode;

/**
 * Get the library version string.
 *
 * Returns a static pointer and must NOT be freed.
 */
const char *docprims_version(void);

/**
 * Get the ABI version number.
 */
uint32_t docprims_abi_version(void);

/**
 * Frees a string allocated by docprims functions.
 *
 * # Safety
 *
 * The pointer must have been returned by a docprims function that allocates
 * strings (e.g., `docprims_extract_file_json`, `docprims_last_error`).
 * Passing null is safe and is a no-op.
 */
void docprims_free_string(char *s);

/**
 * Extract a schema-conformant v0 `DocprimsExtract` JSON string from a file path.
 *
 * # Safety
 *
 * - `path` must be a valid NUL-terminated UTF-8 string
 * - `options_json` may be null; when non-null it must be UTF-8 JSON
 * - `out_json` must be a valid pointer to a `char*` slot
 * - On success, `*out_json` must be freed with `docprims_free_string()`
 */
enum DocprimsErrorCode docprims_extract_file_json(const char *path,
                                                  const char *options_json,
                                                  char **out_json);

/**
 * Extract a schema-conformant v0 `DocprimsExtract` JSON string from in-memory bytes.
 *
 * # Safety
 *
 * - `data` must be non-null and valid for reads of `len` bytes (unless `len == 0`)
 * - `source_uri` must be a valid NUL-terminated UTF-8 string and include a filename-like
 *   extension for format routing (e.g., `mem://upload.docx`)
 * - `options_json` may be null; when non-null it must be UTF-8 JSON
 * - `out_json` must be a valid pointer to a `char*` slot
 * - On success, `*out_json` must be freed with `docprims_free_string()`
 */
enum DocprimsErrorCode docprims_extract_bytes_json(const uint8_t *data,
                                                   uintptr_t len,
                                                   const char *source_uri,
                                                   const char *options_json,
                                                   char **out_json);

/**
 * Clear thread-local error state.
 */
void docprims_clear_error(void);

/**
 * Get the last error code for the current thread.
 */
enum DocprimsErrorCode docprims_last_error_code(void);

/**
 * Get the last error message for the current thread.
 *
 * # Safety
 *
 * The returned pointer must be freed with `docprims_free_string()`.
 */
char *docprims_last_error(void);

#endif  /* DOCPRIMS_H */
