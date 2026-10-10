/* Actual C caller: ownership, malformed inputs, guarded edits and round trips.
 * Build against ffi,write,il. No workspace fixture is needed. */
#include "xgwx.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define BYTES(s) (const uint8_t *)(s), sizeof(s) - 1

static char *json_string(const xgwx_document *doc) {
    xgwx_buffer buffer = {0};
    assert(xgwx_document_inspect_json(doc, &buffer) == XGWX_OK);
    char *text = malloc(buffer.len + 1);
    assert(text);
    memcpy(text, buffer.data, buffer.len);
    text[buffer.len] = 0;
    xgwx_buffer_free(&buffer);
    assert(!buffer.data && !buffer.len);
    xgwx_buffer_free(&buffer);
    return text;
}

int main(void) {
    assert(xgwx_abi_version() == XGWX_ABI_VERSION);
    assert(xgwx_features() == (XGWX_FEATURE_WRITE | XGWX_FEATURE_IL));
    xgwx_document *doc = NULL, *copy = NULL;
    xgwx_buffer bytes = {0}, again = {0}, xml = {0}, il = {0};
    assert(xgwx_document_parse(NULL, 1, &doc) == XGWX_INVALID_ARGUMENT);
    assert(!doc && strlen(xgwx_last_error()) > 0);
    assert(xgwx_document_parse(NULL, 0, &doc) == XGWX_PARSE_ERROR);
    assert(xgwx_document_parse(NULL, SIZE_MAX, &doc) == XGWX_INVALID_ARGUMENT);
    assert(xgwx_document_parse(BYTES("not a workspace"), &doc) == XGWX_PARSE_ERROR);
    assert(xgwx_document_parse(BYTES("x"), NULL) == XGWX_INVALID_ARGUMENT);
    assert(xgwx_document_inspect_json(NULL, &xml) == XGWX_INVALID_ARGUMENT);
    assert(!xml.data && !xml.len);
    assert(xgwx_project_create(BYTES("Unknown CPU"), BYTES("LD"), &doc) == XGWX_OPERATION_ERROR);
    assert(!doc);
    assert(xgwx_project_create(BYTES("XGI-CPUE"), BYTES("ST"), &doc) == XGWX_OK);
    assert(strlen(xgwx_last_error()) == 0);
    xgwx_buffer list = {0}, catalog = {0};
    assert(xgwx_document_list_json(doc, XGWX_LIST_PROGRAMS, &list) == XGWX_OK);
    assert(list.len > 0);
    xgwx_buffer_free(&list);
    assert(xgwx_document_list_json(doc, XGWX_LIST_TEXT_PROGRAMS, &list) == XGWX_OK);
    xgwx_buffer_free(&list);
    assert(xgwx_module_catalog_json(&catalog) == XGWX_OK && catalog.len > 0);
    xgwx_buffer_free(&catalog);
    assert(xgwx_document_apply_edits_json(doc, BYTES("{\"schema_version\":1,\"edits\":[]}")) == XGWX_OK);
    assert(xgwx_document_apply_edits_json(doc, BYTES("not JSON")) == XGWX_INVALID_ARGUMENT);
    xgwx_document *original = doc;
    assert(xgwx_project_create(BYTES("XGI-CPUE"), BYTES("ST"), &doc) == XGWX_INVALID_ARGUMENT);
    assert(doc == original);
    assert(xgwx_document_xml(doc, NULL) == XGWX_INVALID_ARGUMENT);
    assert(xgwx_document_xml(doc, &xml) == XGWX_OK && xml.len > 0);
    uint8_t *old_xml = xml.data;
    assert(xgwx_document_xml(doc, &xml) == XGWX_INVALID_ARGUMENT);
    assert(xml.data == old_xml);
    xgwx_buffer_free(&xml);

    assert(xgwx_document_serialize(doc, &bytes) == XGWX_OK);
    assert(xgwx_document_rename_project(doc, BYTES("Wrong name"), BYTES("New name")) == XGWX_OPERATION_ERROR);
    const uint8_t invalid_utf8[] = {0xff};
    assert(xgwx_document_rename_project(doc, BYTES("NewProject"), invalid_utf8, 1) == XGWX_INVALID_ARGUMENT);
    assert(xgwx_document_select_cpu(doc, BYTES("Unknown CPU")) == XGWX_OPERATION_ERROR);
    assert(xgwx_document_serialize(doc, &again) == XGWX_OK);
    assert(bytes.len == again.len && memcmp(bytes.data, again.data, bytes.len) == 0);
    xgwx_buffer_free(&again);
    xgwx_buffer_free(&bytes);

    assert(xgwx_document_apply_edits_json(doc, BYTES("{\"schema_version\":1,\"edits\":[{\"operation\":\"update_program\",\"program_index\":0,\"expected_object_id\":\"wrong identity\",\"patch\":{\"name\":\"no\"}}]}")) == XGWX_OPERATION_ERROR);
    assert(xgwx_document_rename_project(doc, BYTES("NewProject"), BYTES("C project \xf0\x9f\x98\x80")) == XGWX_OK);
    char *summary = json_string(doc);
    assert(strstr(summary, "\"schema_version\":1"));
    assert(strstr(summary, "C project \xf0\x9f\x98\x80"));
    free(summary);
    assert(xgwx_document_select_cpu(doc, BYTES("XGI-CPUH")) == XGWX_OK);
    assert(xgwx_document_program_il(doc, SIZE_MAX, &il) == XGWX_INVALID_ARGUMENT);
    assert(!il.data && !il.len);
    assert(xgwx_document_serialize(doc, &bytes) == XGWX_OK);
    assert(xgwx_document_parse(bytes.data, bytes.len, &copy) == XGWX_OK);
    assert(xgwx_document_serialize(copy, &again) == XGWX_OK);
    assert(bytes.len == again.len && memcmp(bytes.data, again.data, bytes.len) == 0);
    summary = json_string(copy);
    assert(strstr(summary, "C project \xf0\x9f\x98\x80"));
    free(summary);
    xgwx_buffer_free(&bytes);
    xgwx_buffer_free(&again);
    xgwx_document_free(copy);
    xgwx_document_free(doc);
    xgwx_document_free(NULL);
    xgwx_buffer_free(NULL);
    puts("C ABI smoke tests passed");
    return 0;
}
