/* Exercise the same public symbols against independently enabled features. */
#include "xgwx.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

#ifndef EXPECTED_FEATURES
#define EXPECTED_FEATURES 0u
#endif
#define BYTES(s) (const uint8_t *)(s), sizeof(s) - 1

int main(int argc, char **argv) {
    assert(argc == 2);
    assert(xgwx_features() == EXPECTED_FEATURES);
    FILE *file = fopen(argv[1], "rb");
    assert(file && fseek(file, 0, SEEK_END) == 0);
    long size = ftell(file);
    assert(size > 0 && fseek(file, 0, SEEK_SET) == 0);
    uint8_t *input = malloc((size_t)size);
    assert(input && fread(input, 1, (size_t)size, file) == (size_t)size);
    assert(fclose(file) == 0);
    xgwx_document *doc = NULL, *created = NULL;
    xgwx_buffer xml = {0}, json = {0}, serialized = {0}, il = {0};
    assert(xgwx_document_parse(input, (size_t)size, &doc) == XGWX_OK);
    free(input);
    assert(xgwx_document_xml(doc, &xml) == XGWX_OK && xml.len > 0);
    assert(xgwx_document_inspect_json(doc, &json) == XGWX_OK && json.len > 0);
    xgwx_buffer list = {0}, catalog = {0};
    assert(xgwx_document_list_json(doc, XGWX_LIST_PROGRAMS, &list) == XGWX_OK && list.len > 0);
    xgwx_buffer_free(&list);
    if (EXPECTED_FEATURES & XGWX_FEATURE_WRITE) {
        assert(xgwx_document_serialize(doc, &serialized) == XGWX_OK);
        assert(xgwx_project_create(BYTES("XGI-CPUE"), BYTES("ST"), &created) == XGWX_OK);
        assert(xgwx_module_catalog_json(&catalog) == XGWX_OK);
        assert(xgwx_document_apply_edits_json(doc, BYTES("{\"schema_version\":1,\"edits\":[]}")) == XGWX_OK);
    } else {
        assert(xgwx_document_serialize(doc, &serialized) == XGWX_FEATURE_UNAVAILABLE);
        assert(xgwx_project_create(BYTES("XGI-CPUE"), BYTES("ST"), &created) == XGWX_FEATURE_UNAVAILABLE);
        assert(xgwx_document_rename_project(doc, BYTES("name"), BYTES("new")) == XGWX_FEATURE_UNAVAILABLE);
        assert(xgwx_document_select_cpu(doc, BYTES("XGI-CPUE")) == XGWX_FEATURE_UNAVAILABLE);
        assert(xgwx_module_catalog_json(&catalog) == XGWX_FEATURE_UNAVAILABLE);
        assert(xgwx_document_module_options_json(doc, 0, 0, &list) == XGWX_FEATURE_UNAVAILABLE);
        assert(xgwx_document_apply_edits_json(doc, BYTES("{\"schema_version\":1,\"edits\":[]}")) == XGWX_FEATURE_UNAVAILABLE);
        assert(!serialized.data && !created);
    }
    if (EXPECTED_FEATURES & XGWX_FEATURE_IL) {
        assert(xgwx_document_program_il(doc, 0, &il) == XGWX_OK);
    } else {
        assert(xgwx_document_program_il(doc, 0, &il) == XGWX_FEATURE_UNAVAILABLE);
        assert(!il.data);
    }
    xgwx_document_free(doc);
    xgwx_document_free(created);
    /* Returned buffers remain owned independently after document release. */
    assert(xml.len && xml.data[0] == '<');
    assert(json.len && json.data[0] == '{');
    xgwx_buffer_free(&xml);
    xgwx_buffer_free(&json);
    xgwx_buffer_free(&serialized);
    xgwx_buffer_free(&il);
    xgwx_buffer_free(&catalog);
    puts("C ABI feature checks passed");
    return 0;
}
