#ifndef XGWX_H
#define XGWX_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ABI v1. Enable Rust feature "ffi" to export these symbols. */
#define XGWX_ABI_VERSION 1u
typedef int32_t xgwx_status;
#define XGWX_OK                  ((xgwx_status)0)
#define XGWX_INVALID_ARGUMENT    ((xgwx_status)1)
#define XGWX_PARSE_ERROR         ((xgwx_status)2)
#define XGWX_OPERATION_ERROR     ((xgwx_status)3)
#define XGWX_FEATURE_UNAVAILABLE ((xgwx_status)4)
#define XGWX_PANIC               ((xgwx_status)255)
#define XGWX_FEATURE_WRITE 1u
#define XGWX_FEATURE_IL    2u
#define XGWX_LIST_MODULES        1u
#define XGWX_LIST_NETWORKS       2u
#define XGWX_LIST_PROGRAMS       3u
#define XGWX_LIST_VARIABLES      4u
#define XGWX_LIST_NETWORK_MODULES 5u
#define XGWX_LIST_TEXT_PROGRAMS  6u

typedef struct xgwx_document xgwx_document;

/* Returned bytes belong to Rust and are NOT NUL-terminated. Initialize to {0}.
 * Release with xgwx_buffer_free, never free/realloc. Do not copy ownership,
 * modify the pointer/length, or access bytes after release. */
typedef struct xgwx_buffer {
    uint8_t *data;
    size_t len;
} xgwx_buffer;

/* Handles must not be used concurrently, including while being freed.
 * Inputs are copied or consumed synchronously and never retained. Each nonempty
 * input span must point to len readable bytes; NULL is allowed for zero length.
 * Text inputs are UTF-8 byte spans, not C strings. Output handles must initially
 * be NULL and output buffers empty. Failures preserve outputs and documents.
 * The ABI performs no filesystem I/O; callers choose their own file handling. */
uint32_t xgwx_abi_version(void);
uint32_t xgwx_features(void);

/* Borrowed, NUL-terminated UTF-8; empty after success. Valid until the next
 * fallible call on this thread or thread exit. Copy before another call.
 * Version/features/free functions leave the last error unchanged. */
const char *xgwx_last_error(void);

xgwx_status xgwx_document_parse(const uint8_t *data, size_t len,
                                xgwx_document **out);
/* Release exactly once. NULL is a no-op; discard the handle after release. */
void xgwx_document_free(xgwx_document *doc);
/* Resets the struct to {NULL, 0}; repeated calls on that struct are safe. */
void xgwx_buffer_free(xgwx_buffer *buffer);

/* UTF-8 JSON, schema_version 1, snake_case field names. Sections: project,
 * configurations, bases, modules, tasks, programs, networks, network_modules,
 * variables, variables_error, program_languages. A variables decoding error
 * sets variables to null and variables_error to a message. */
xgwx_status xgwx_document_inspect_json(const xgwx_document *doc, xgwx_buffer *out);
/* Full decompressed XML in UTF-8. */
xgwx_status xgwx_document_xml(const xgwx_document *doc, xgwx_buffer *out);
/* Dedicated lists: {"schema_version":1,"items":[...]}. See selectors above.
 * Undecodable globals return an error; inspect_json offers partial inspection. */
xgwx_status xgwx_document_list_json(const xgwx_document *doc, uint32_t list,
                                   xgwx_buffer *out);

/* Requires "write". Catalog includes verified options and all visible fields. */
xgwx_status xgwx_module_catalog_json(xgwx_buffer *out);
xgwx_status xgwx_document_module_options_json(const xgwx_document *doc,
                                            uint32_t base, uint32_t slot,
                                            xgwx_buffer *out);
/* Atomic batch: {"schema_version":1,"edits":[{"operation":..., ...}]}.
 * UTF-8 JSON, strict snake_case keys. See docs/c-api.md for operation schemas.
 * All guards and final serialization must succeed before any change is committed. */
xgwx_status xgwx_document_apply_edits_json(xgwx_document *doc,
                                         const uint8_t *json, size_t len);

/* Requires feature "write". Output is a verified .xgwx byte container. */
xgwx_status xgwx_document_serialize(const xgwx_document *doc, xgwx_buffer *out);
xgwx_status xgwx_project_create(const uint8_t *cpu, size_t cpu_len,
                               const uint8_t *language, size_t language_len,
                               xgwx_document **out);
xgwx_status xgwx_document_rename_project(xgwx_document *doc,
                                        const uint8_t *expected, size_t expected_len,
                                        const uint8_t *name, size_t name_len);
xgwx_status xgwx_document_select_cpu(xgwx_document *doc,
                                    const uint8_t *cpu, size_t cpu_len);

/* Requires feature "il". Program indices use inspection's document order.
 * Unknown or unsupported ladder layouts return an error. Output is UTF-8. */
xgwx_status xgwx_document_program_il(const xgwx_document *doc,
                                    size_t program_index, xgwx_buffer *out);

#ifdef __cplusplus
}
#endif
#endif
