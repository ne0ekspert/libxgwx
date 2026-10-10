/* Read a workspace through the C ABI and print its JSON summary.
 * Usage: inspect-c INPUT [NEW_OUTPUT EXPECTED_NAME NEW_NAME]
 * Optional renaming requires feature "write". Output creation is exclusive;
 * an existing file is never replaced. See docs/c-api.md for build commands. */
#include "xgwx.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int check(xgwx_status status) {
    if (status == XGWX_OK) return 1;
    fprintf(stderr, "libxgwx error %d: %s\n", (int)status, xgwx_last_error());
    return 0;
}

int main(int argc, char **argv) {
    int result = EXIT_FAILURE;
    FILE *file = NULL;
    uint8_t *input = NULL;
    xgwx_document *doc = NULL;
    xgwx_buffer json = {0}, output = {0};

    if (argc != 2 && argc != 5) {
        fprintf(stderr, "usage: %s INPUT [NEW_OUTPUT EXPECTED_NAME NEW_NAME]\n", argv[0]);
        return result;
    }
    if (xgwx_abi_version() != XGWX_ABI_VERSION) {
        fprintf(stderr, "incompatible libxgwx ABI\n");
        return result;
    }
    file = fopen(argv[1], "rb");
    if (!file) { perror("open input"); goto cleanup; }
    if (fseek(file, 0, SEEK_END) != 0) { perror("seek input"); goto cleanup; }
    long size = ftell(file);
    if (size <= 0 || (uintmax_t)size > SIZE_MAX) {
        fprintf(stderr, "input size is invalid\n");
        goto cleanup;
    }
    if (fseek(file, 0, SEEK_SET) != 0) { perror("seek input"); goto cleanup; }
    input = malloc((size_t)size);
    if (!input) { perror("allocate input"); goto cleanup; }
    if (fread(input, 1, (size_t)size, file) != (size_t)size) {
        fprintf(stderr, "failed to read input\n");
        goto cleanup;
    }
    fclose(file);
    file = NULL;
    if (!check(xgwx_document_parse(input, (size_t)size, &doc))) goto cleanup;
    free(input); /* The handle retained its own copy. */
    input = NULL;

    if (argc == 5) {
        if (!check(xgwx_document_rename_project(
                doc, (const uint8_t *)argv[3], strlen(argv[3]),
                (const uint8_t *)argv[4], strlen(argv[4])))) goto cleanup;
        if (!check(xgwx_document_serialize(doc, &output))) goto cleanup;
        file = fopen(argv[2], "wbx");
        if (!file) { perror("create output"); goto cleanup; }
        if (fwrite(output.data, 1, output.len, file) != output.len) {
            fprintf(stderr, "failed to write output\n");
            goto cleanup;
        }
        int close_result = fclose(file);
        file = NULL;
        if (close_result != 0) { perror("close output"); goto cleanup; }
    }
    if (!check(xgwx_document_inspect_json(doc, &json))) goto cleanup;
    if (fwrite(json.data, 1, json.len, stdout) != json.len || putchar('\n') == EOF) {
        fprintf(stderr, "failed to print summary\n");
        goto cleanup;
    }
    result = EXIT_SUCCESS;
cleanup:
    if (file) fclose(file);
    free(input);
    xgwx_buffer_free(&json);
    xgwx_buffer_free(&output);
    xgwx_document_free(doc);
    return result;
}
