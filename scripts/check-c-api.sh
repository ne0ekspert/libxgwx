#!/bin/sh
# Native Linux C ABI checks. Run from any directory; no files outside the build
# directory and a disposable temporary directory are modified.
set -eu
repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$repo_root"
task_ffi_target=${CARGO_TARGET_DIR:-"$repo_root/target"}
mkdir -p "$task_ffi_target"
task_ffi_target=$(CDPATH= cd -- "$task_ffi_target" && pwd)
task_ffi_bin=$(mktemp -d)
trap 'rm -rf "$task_ffi_bin"' EXIT HUP INT TERM

"${CARGO:-cargo}" build --locked --lib --features ffi,write,il --target-dir "$task_ffi_target"
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -Iinclude tests/ffi_smoke.c \
  -L"$task_ffi_target/debug" -lxgwx -Wl,-rpath,"$task_ffi_target/debug" -o "$task_ffi_bin/smoke"
"$task_ffi_bin/smoke"
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -Iinclude tests/ffi_smoke.c \
  "$task_ffi_target/debug/libxgwx.a" -ldl -lpthread -lm -o "$task_ffi_bin/static-smoke"
"$task_ffi_bin/static-smoke"
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -Iinclude examples/c/inspect.c \
  -L"$task_ffi_target/debug" -lxgwx -Wl,-rpath,"$task_ffi_target/debug" -o "$task_ffi_bin/inspect"
"$task_ffi_bin/inspect" fixtures/empty-projects/new-xgk.xgwx > "$task_ffi_bin/summary.json"
"$task_ffi_bin/inspect" fixtures/empty-projects/new-xgk.xgwx \
  "$task_ffi_bin/renamed.xgwx" NewProject "C API example" > "$task_ffi_bin/renamed.json"
"$task_ffi_bin/inspect" "$task_ffi_bin/renamed.xgwx" > "$task_ffi_bin/reparsed.json"
if "$task_ffi_bin/inspect" fixtures/empty-projects/new-xgk.xgwx \
  "$task_ffi_bin/renamed.xgwx" NewProject "Must not overwrite" 2>/dev/null; then
  echo "C example unexpectedly replaced an existing output" >&2
  exit 1
fi

"${CXX:-c++}" -std=c++11 -Wall -Wextra -Werror -Iinclude -x c++ -fsyntax-only - <<'CPP'
#include "xgwx.h"
int main() { xgwx_buffer buffer = {nullptr, 0}; xgwx_buffer_free(&buffer); }
CPP

for task_ffi_case in 'ffi:0' 'ffi,il:2' 'ffi,write:1'; do
  task_ffi_features=${task_ffi_case%:*}
  task_ffi_expected=${task_ffi_case#*:}
  "${CARGO:-cargo}" build --locked --lib --features "$task_ffi_features" --target-dir "$task_ffi_target"
  "${CC:-cc}" -std=c11 -Wall -Wextra -Werror -Iinclude \
    -DEXPECTED_FEATURES="$task_ffi_expected" tests/ffi_features.c \
    -L"$task_ffi_target/debug" -lxgwx -Wl,-rpath,"$task_ffi_target/debug" -o "$task_ffi_bin/features"
  "$task_ffi_bin/features" fixtures/ladder-edit/linear.xgwx
done
# Leave the build directory with the complete native API enabled.
"${CARGO:-cargo}" build --locked --lib --features ffi,write,il --target-dir "$task_ffi_target"
