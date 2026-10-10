#!/usr/bin/env python3
"""Load native lists and optionally apply a guarded JSON edit batch via ctypes.

Usage: python3 examples/python/lists.py LIBRARY WORKSPACE [EDIT_JSON NEW_OUTPUT]
Requires a shared library built with ffi (and write when editing). Uses only the
Python standard library. The output path must not exist; the input is unchanged.
"""
import ctypes as c
import json
from pathlib import Path
import sys


class Buffer(c.Structure):
    _fields_ = [("data", c.c_void_p), ("len", c.c_size_t)]


class Document:
    def __init__(self, library, data):
        self.lib = c.CDLL(str(Path(library).resolve()))
        self.handle = c.c_void_p()
        signatures = {
            "xgwx_abi_version": ([], c.c_uint32),
            "xgwx_last_error": ([], c.c_char_p),
            "xgwx_document_parse": ([c.c_void_p, c.c_size_t, c.POINTER(c.c_void_p)], c.c_int32),
            "xgwx_document_list_json": ([c.c_void_p, c.c_uint32, c.POINTER(Buffer)], c.c_int32),
            "xgwx_document_apply_edits_json": ([c.c_void_p, c.c_void_p, c.c_size_t], c.c_int32),
            "xgwx_document_serialize": ([c.c_void_p, c.POINTER(Buffer)], c.c_int32),
            "xgwx_buffer_free": ([c.POINTER(Buffer)], None),
            "xgwx_document_free": ([c.c_void_p], None),
        }
        for name, (args, result) in signatures.items():
            function = getattr(self.lib, name)
            function.argtypes, function.restype = args, result
        if self.lib.xgwx_abi_version() != 1:
            raise RuntimeError("incompatible libxgwx ABI")
        self.check(self.lib.xgwx_document_parse(data, len(data), c.byref(self.handle)))

    def check(self, status):
        if status:
            # Copy this thread's message before any other fallible native call.
            message = self.lib.xgwx_last_error().decode("utf-8")
            raise RuntimeError(f"libxgwx error {status}: {message}")

    def output(self, function, *args):
        buffer = Buffer()
        try:
            self.check(function(self.handle, *args, c.byref(buffer)))
            return c.string_at(buffer.data, buffer.len) if buffer.len else b""
        finally:
            self.lib.xgwx_buffer_free(c.byref(buffer))

    def lists(self):
        result = {}
        for selector, name in enumerate(
            ("modules", "networks", "programs", "variables", "network_modules", "text_programs"), 1
        ):
            try:
                result[name] = json.loads(self.output(self.lib.xgwx_document_list_json, selector))
            except RuntimeError as error:
                result[name] = {"error": str(error)}
        return result

    def apply(self, request):
        self.check(self.lib.xgwx_document_apply_edits_json(self.handle, request, len(request)))

    def close(self):
        if self.handle.value:
            self.lib.xgwx_document_free(self.handle)
            self.handle = c.c_void_p()


def main():
    if len(sys.argv) not in (3, 5):
        raise SystemExit("usage: lists.py LIBRARY WORKSPACE [EDIT_JSON NEW_OUTPUT]")
    doc = Document(sys.argv[1], Path(sys.argv[2]).read_bytes())
    try:
        if len(sys.argv) == 5:
            doc.apply(Path(sys.argv[3]).read_bytes())
            output = doc.output(doc.lib.xgwx_document_serialize)
            with Path(sys.argv[4]).open("xb") as file:
                file.write(output)
        print(json.dumps(doc.lists(), ensure_ascii=False, indent=2))
    finally:
        doc.close()


if __name__ == "__main__":
    main()
