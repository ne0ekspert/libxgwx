#include "xgwx.h"
#include <assert.h>
#include <string.h>
#include <stdio.h>
#define SPAN(s) (const uint8_t *)(s), sizeof(s)-1
#define OK(call) do { xgwx_status s=(call); if(s!=XGWX_OK){fprintf(stderr,"%s\n",xgwx_last_error());return 1;} }while(0)
int main(void) {
    xgwx_document *doc=NULL,*opened=NULL;
    xgwx_buffer bytes={0},status={0};
    OK(xgwx_project_create(SPAN("XGI-CPUE"),SPAN("ST"),&doc));
    OK(xgwx_document_set_program_password(doc,0,NULL,0,SPAN("first")));
    OK(xgwx_document_set_project_password(doc,NULL,0,SPAN("project1")));
    OK(xgwx_document_serialize(doc,&bytes));
    assert(xgwx_document_parse(bytes.data,bytes.len,&opened)==XGWX_PASSWORD_REQUIRED);
    assert(opened==NULL);
    assert(xgwx_document_parse_with_password(bytes.data,bytes.len,SPAN("wrong"),&opened)==XGWX_PASSWORD_INCORRECT);
    OK(xgwx_document_parse_with_password(bytes.data,bytes.len,SPAN("project1"),&opened));
    assert(xgwx_document_unlock_program(opened,0,SPAN("wrong"))==XGWX_PASSWORD_INCORRECT);
    OK(xgwx_document_unlock_program(opened,0,SPAN("first")));
    OK(xgwx_document_passwords_json(opened,&status));
    assert(status.len>0);xgwx_buffer_free(&status);
    OK(xgwx_document_set_program_password(opened,0,SPAN("first"),SPAN("second")));
    OK(xgwx_document_set_program_password(opened,0,SPAN("second"),NULL,0));
    OK(xgwx_document_set_project_password(opened,SPAN("project1"),NULL,0));
    xgwx_buffer_free(&bytes);OK(xgwx_document_serialize(opened,&bytes));
    xgwx_document_free(doc);doc=NULL;
    OK(xgwx_document_parse(bytes.data,bytes.len,&doc));
    xgwx_document_free(doc);xgwx_document_free(opened);xgwx_buffer_free(&bytes);
    puts("C password lifecycle passed");return 0;
}
