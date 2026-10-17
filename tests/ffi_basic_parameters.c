/* Compile against the public header and exercise the real JSON ABI. */
#include "xgwx.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static xgwx_status apply(xgwx_document *doc, const char *json) {
    return xgwx_document_apply_edits_json(doc, (const uint8_t *)json, strlen(json));
}
static char *inspect(xgwx_document *doc) {
    const char *request="{\"section\":\"parameters\",\"offset\":0,\"limit\":1}";
    xgwx_buffer out={0};
    assert(xgwx_document_inspect_section_json(doc,(const uint8_t *)request,strlen(request),&out)==XGWX_OK);
    char *text=calloc(out.len+1,1); assert(text);
    memcpy(text,out.data,out.len); xgwx_buffer_free(&out); return text;
}
int main(void) {
    const char *models[]={"xgk","xgi"};
    for(size_t i=0;i<2;++i) {
        char path[256];
        snprintf(path,sizeof(path),"fixtures/basic-parameters/%s-off-resaved.xgwx",models[i]);
        FILE *f=fopen(path,"rb"); assert(f);
        assert(fseek(f,0,SEEK_END)==0); long size=ftell(f); assert(size>0); rewind(f);
        unsigned char *bytes=malloc((size_t)size); assert(bytes);
        assert(fread(bytes,1,(size_t)size,f)==(size_t)size); fclose(f);
        xgwx_document *doc=NULL,*reopened=NULL;
        assert(xgwx_document_parse(bytes,(size_t)size,&doc)==XGWX_OK); free(bytes);
        char *before=inspect(doc);
        assert(strstr(before,"\"parameter_index\":0"));
        assert(strstr(before,"basic_operation_fields"));
        assert(strstr(before,"\"checked\":0,\"kind\":\"checkbox\",\"unchecked\":43536"));
        const char *field=i==0?"SCAN_WD_TIME":"SCAN_WD_TIME_0";
        /* Native off fixtures retain the lower watchdog boundary of 10 ms. */
        char request[1024];
        snprintf(request,sizeof(request),"{\"schema_version\":1,\"edits\":[{\"operation\":\"edit_basic_operation\",\"parameter_index\":0,\"changes\":[{\"name\":\"%s\",\"expected\":\"10\",\"replacement\":650}]}]}",field);
        if(apply(doc,request)!=XGWX_OK) { fprintf(stderr,"%s\n",xgwx_last_error()); abort(); }
        assert(apply(doc,request)==XGWX_OPERATION_ERROR);
        char *after=inspect(doc);assert(strcmp(before,after));assert(strstr(after,"\"value\":\"650\""));free(before);
        xgwx_buffer serialized={0};
        assert(xgwx_document_serialize(doc,&serialized)==XGWX_OK);
        assert(xgwx_document_parse(serialized.data,serialized.len,&reopened)==XGWX_OK);
        char *again=inspect(reopened);assert(!strcmp(after,again));free(after);free(again);
        xgwx_buffer_free(&serialized);xgwx_document_free(doc);xgwx_document_free(reopened);
    }
    puts("C basic parameter inspection, edit, stale guard and reparse passed");
    return 0;
}
