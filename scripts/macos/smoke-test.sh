#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
    echo "usage: $0 <salmon-binary> <examples-dir>" >&2
    exit 2
fi

salmon_binary="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
examples_source="$2"
failures=0

workspace="$(mktemp -d)"
trap 'rm -rf "$workspace"' EXIT

examples="$workspace/carpeta con espacios/ñandú"
mkdir -p "$examples"
cp "$examples_source"/*.salmon "$examples/"
perl -pe 's/\n/\r\n/' "$examples/dfa.salmon" > "$examples/dfa crlf.salmon"
printf 'dfa broken {\n    initial: q0\n    final: q0\n    q0: a=q0, eps=q0\n}\n' > "$examples/inválido.salmon"

expect() {
    local description="$1" expected_status="$2" expected_stdout="$3"
    shift 3
    local actual_stdout actual_status
    set +e
    actual_stdout="$("$@" 2>/dev/null)"
    actual_status=$?
    set -e
    if [[ "$actual_status" == "$expected_status" && "$actual_stdout" == "$expected_stdout" ]]; then
        echo "ok   $description"
    else
        echo "FAIL $description: status $actual_status (expected $expected_status), stdout '$actual_stdout' (expected '$expected_stdout')"
        failures=$((failures + 1))
    fi
}

expect_repl() {
    local description="$1" input="$2" expected_stdout="$3"
    local actual_stdout actual_status
    set +e
    actual_stdout="$(printf '%b' "$input" | "$salmon_binary" run "$examples/dfa.salmon" 2>/dev/null)"
    actual_status=$?
    set -e
    if [[ "$actual_status" == 0 && "$actual_stdout" == "$expected_stdout" ]]; then
        echo "ok   $description"
    else
        echo "FAIL $description: status $actual_status, stdout '$actual_stdout' (expected '$expected_stdout')"
        failures=$((failures + 1))
    fi
}

expect "usage without arguments" 2 "" "$salmon_binary"
expect "check valid dfa" 0 "" "$salmon_binary" check "$examples/dfa.salmon"
expect "check valid nfa" 0 "" "$salmon_binary" check "$examples/nfa.salmon"
expect "check valid enfa" 0 "" "$salmon_binary" check "$examples/enfa.salmon"
expect "check invalid file in unicode path" 1 "" "$salmon_binary" check "$examples/inválido.salmon"
expect "check missing file" 2 "" "$salmon_binary" check "$examples/no existe.salmon"
expect "check crlf file" 0 "" "$salmon_binary" check "$examples/dfa crlf.salmon"
expect "run accepted word" 0 "OK" "$salmon_binary" run "$examples/dfa.salmon" 11
expect "run rejected word" 1 "NOT" "$salmon_binary" run "$examples/dfa.salmon" 1
expect "run empty word" 0 "OK" "$salmon_binary" run "$examples/dfa.salmon" ""
expect "run symbol outside the alphabet" 1 "NOT" "$salmon_binary" run "$examples/dfa.salmon" 2
expect "run unclosed quoted symbol" 2 "" "$salmon_binary" run "$examples/dfa.salmon" '"11'
expect "run crlf file" 0 "OK" "$salmon_binary" run "$examples/dfa crlf.salmon" 11
expect "run nfa" 0 "OK" "$salmon_binary" run "$examples/nfa.salmon" baba
expect "run enfa" 0 "OK" "$salmon_binary" run "$examples/enfa.salmon" a
expect_repl "repl session" '11\n1\n\n"11\n:quit\n' $'Salmon — even_ones\n> OK\n> NOT\n> OK\n> ERROR: unclosed quoted symbol\n> '
expect_repl "repl crlf input" '11\r\n1\r\n:quit\r\n' $'Salmon — even_ones\n> OK\n> NOT\n> '
expect_repl "repl end of input" '11\n' $'Salmon — even_ones\n> OK\n> '

if [[ $failures -ne 0 ]]; then
    echo "$failures smoke test(s) failed" >&2
    exit 1
fi
echo "all smoke tests passed on $(uname -m), macOS $(sw_vers -productVersion)"
