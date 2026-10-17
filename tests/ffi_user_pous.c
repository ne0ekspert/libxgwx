/* Real C ABI lifecycle, complete snapshot guards, rollback and reparse. */
#include "xgwx.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static xgwx_status apply(xgwx_document *d,const char *s) {
    xgwx_status status=xgwx_document_apply_edits_json(d,(const uint8_t *)s,strlen(s));
    if(status!=XGWX_OK) fprintf(stderr,"%s\n",xgwx_last_error());
    return status;
}
#define APPLY(d,s) apply(d,s)
static char *snapshot(xgwx_document *doc) {
    xgwx_buffer b={0};
    assert(xgwx_document_list_json(doc,XGWX_LIST_USER_POUS,&b)==XGWX_OK);
    char *json=calloc(b.len+1,1); assert(json);
    memcpy(json,b.data,b.len); xgwx_buffer_free(&b);
    char *first=strchr(json,'['), *last=strrchr(json,']');
    assert(first && last && first+1<last);
    *last=0;
    char *result=malloc(strlen(first+1)+1); assert(result);
    strcpy(result,first+1); free(json); return result;
}
int main(void) {
    xgwx_document *doc=NULL,*copy=NULL;
    const char *cpu="XGI-CPUE",*lang="ST";
    assert(xgwx_project_create((const uint8_t *)cpu,strlen(cpu),(const uint8_t *)lang,strlen(lang),&doc)==XGWX_OK);
    const char *create="{\"schema_version\":1,\"edits\":[{\"operation\":\"create_user_pou\",\"patch\":{\"name\":\"Scale\",\"kind\":\"function\",\"object_id\":\"f0000000-0000-4000-8000-000000000001\",\"symbol_id\":\"f0000000-0000-4000-8000-000000000002\",\"return_type\":\"DINT\",\"variables\":[],\"source\":\"Scale := 1;\",\"comment\":\"C ABI\"}}]}";
    assert(APPLY(doc,create)==XGWX_OK);
    char *expected=snapshot(doc); assert(strstr(expected,"\"data_type\":\"DINT\""));
    size_t capacity=2*strlen(expected)+1024;
    char *edit=malloc(capacity); assert(edit);
    snprintf(edit,capacity,"{\"schema_version\":1,\"edits\":[{\"operation\":\"edit_user_pou_source\",\"expected\":%s,\"source\":\"Scale := 2;\"}]}",expected);
    assert(APPLY(doc,edit)==XGWX_OK);
    assert(APPLY(doc,edit)==XGWX_OPERATION_ERROR); /* old source snapshot */
    free(expected); expected=snapshot(doc); assert(strstr(expected,"Scale := 2;"));
    snprintf(edit,capacity,"{\"schema_version\":1,\"edits\":[{\"operation\":\"edit_user_pou_variables\",\"expected\":%s,\"variables\":[{\"name\":\"Scale\",\"scope\":\"return\",\"data_type\":\"DINT\",\"description\":\"\"},{\"name\":\"Scratch\",\"scope\":\"local\",\"data_type\":\"DINT\",\"description\":\"private\"}]}]}",expected);
    assert(APPLY(doc,edit)==XGWX_OK);
    free(expected); expected=snapshot(doc);
    snprintf(edit,capacity,"{\"schema_version\":1,\"edits\":[{\"operation\":\"delete_user_pou\",\"expected\":%s},{\"operation\":\"delete_user_pou\",\"expected\":%s}]}",expected,expected);
    /* Ensure capacity holds two snapshots as the interface grows. */
    assert(strlen(expected)*2+200<capacity);
    assert(APPLY(doc,edit)==XGWX_OPERATION_ERROR);
    char *retained=snapshot(doc); assert(!strcmp(expected,retained)); free(retained);
    xgwx_buffer bytes={0};
    assert(xgwx_document_serialize(doc,&bytes)==XGWX_OK);
    assert(xgwx_document_parse(bytes.data,bytes.len,&copy)==XGWX_OK);
    retained=snapshot(copy); assert(!strcmp(expected,retained));free(retained);
    xgwx_buffer_free(&bytes); xgwx_document_free(copy);
    snprintf(edit,capacity,"{\"schema_version\":1,\"edits\":[{\"operation\":\"delete_user_pou\",\"expected\":%s}]}",expected);
    assert(APPLY(doc,edit)==XGWX_OK);
    assert(xgwx_document_list_json(doc,XGWX_LIST_USER_POUS,&bytes)==XGWX_OK);
    assert(bytes.len>0); xgwx_buffer_free(&bytes);
    const char *block="{\"schema_version\":1,\"edits\":[{\"operation\":\"create_user_pou\",\"patch\":{\"name\":\"Counter\",\"kind\":\"function_block\",\"object_id\":\"f0000000-0000-4000-8000-000000000003\",\"symbol_id\":\"f0000000-0000-4000-8000-000000000004\",\"return_type\":null,\"variables\":[],\"source\":\"(* block *)\"}}]}";
    assert(APPLY(doc,block)==XGWX_OK);
    char *fb=snapshot(doc);assert(strstr(fb,"function_block"));free(fb);
    free(expected);free(edit);xgwx_document_free(doc);
    puts("C user POU lifecycle passed");
}
