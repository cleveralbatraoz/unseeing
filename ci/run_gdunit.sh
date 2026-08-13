#!/bin/sh
# Run gdUnit4 only when its terminal summary agrees with the authored source.
#
# Usage:
#   ci/run_gdunit.sh <godot-project-dir> <runner> [runner arguments ...]
#
# gdUnit4 has historically returned success after a script parse failure made
# part of the requested test tree undiscoverable.  Its exit status is therefore
# necessary but not sufficient.  This adapter derives the expected suite and
# case totals from the source tree before launch, then requires exactly one
# zero-error terminal summary and exact executed-suite/executed-case ratios.
# Pure POSIX sh; the runner is an explicit dependency so the predicate can be
# tested without launching Godot.
set -eu

if [ "$#" -lt 2 ]; then
	echo "usage: $0 <godot-project-dir> <runner> [runner arguments ...]" >&2
	exit 2
fi

PROJECT_DIR="$1"
shift
TEST_DIR="$PROJECT_DIR/tests"

if [ ! -d "$TEST_DIR" ]; then
	echo "ci: gdUnit source census FAILED: missing test directory $TEST_DIR" >&2
	exit 2
fi

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT INT TERM HUP
SUITES="$WORK/suites"
RAW_OUTPUT="$WORK/raw-output"
CLEAN_OUTPUT="$WORK/clean-output"

# The project convention and gdUnit entry point agree that authored suites end
# in _test.gd.  Count files independently of parser success so an unparseable
# suite remains part of the expectation instead of disappearing with the
# runner's discovery result.
find "$TEST_DIR" -type f -name '*_test.gd' -print | LC_ALL=C sort >"$SUITES"
SUITE_COUNT="$(awk 'END { print NR + 0 }' "$SUITES")"
if [ "$SUITE_COUNT" -eq 0 ]; then
	echo "ci: gdUnit source census FAILED: no *_test.gd suites under $TEST_DIR" >&2
	exit 1
fi

CASE_COUNT=0
while IFS= read -r SUITE; do
	SUITE_CASES="$(LC_ALL=C awk '
		/^[[:space:]]*func[[:space:]]+test_[A-Za-z0-9_]*[[:space:]]*\(/ { count += 1 }
		END { print count + 0 }
	' "$SUITE")"
	if [ "$SUITE_CASES" -eq 0 ]; then
		echo "ci: gdUnit source census FAILED: $SUITE declares no test_ functions" >&2
		exit 1
	fi
	CASE_COUNT=$((CASE_COUNT + SUITE_CASES))
done <"$SUITES"

echo "ci: gdUnit source census: $SUITE_COUNT suites, $CASE_COUNT cases"

RUN_STATUS=0
"$@" >"$RAW_OUTPUT" 2>&1 || RUN_STATUS=$?
cat "$RAW_OUTPUT"
if [ "$RUN_STATUS" -ne 0 ]; then
	echo "ci: gdUnit summary FAILED: gdUnit4 exited with status $RUN_STATUS" >&2
	exit 1
fi

# The console reporter colours every fragment independently.  Remove its ANSI
# CSI controls (colour, clear-screen, and cursor-home are the forms it emits)
# and CR bytes before asking for byte-exact logical records.
ESC="$(printf '\033')"
LC_ALL=C sed "s/${ESC}\\[[0-9;]*[A-Za-z]//g" "$RAW_OUTPUT" |
	tr -d '\r' >"$CLEAN_OUTPUT"

FAIL=0
require_record() { # require_record <display-name> <opening-regex> <exact-record>
	NAME="$1"
	OPENING="$2"
	EXPECTED="$3"
	FOUND="$(grep -c "$OPENING" "$CLEAN_OUTPUT" || true)"
	if [ "$FOUND" -eq 0 ]; then
		echo "ci: gdUnit summary FAILED: missing $NAME record" >&2
		FAIL=1
		return
	fi
	if [ "$FOUND" -ne 1 ]; then
		echo "ci: gdUnit summary FAILED: found $FOUND $NAME records; expected exactly one" >&2
		grep "$OPENING" "$CLEAN_OUTPUT" | sed 's/^/ci:   observed: /' >&2 || true
		FAIL=1
		return
	fi
	if [ "$(grep -Fxc "$EXPECTED" "$CLEAN_OUTPUT" || true)" -ne 1 ]; then
		echo "ci: gdUnit summary FAILED: expected exactly: $EXPECTED" >&2
		grep "$OPENING" "$CLEAN_OUTPUT" | sed 's/^/ci:   observed: /' >&2 || true
		FAIL=1
	fi
}

require_record \
	"Overall Summary" \
	'^Overall Summary:' \
	"Overall Summary: $CASE_COUNT test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |"
require_record \
	"Executed test suites" \
	'^Executed test suites:' \
	"Executed test suites: ($SUITE_COUNT/$SUITE_COUNT)"
require_record \
	"Executed test cases" \
	'^Executed test cases :' \
	"Executed test cases : ($CASE_COUNT/$CASE_COUNT)"

if [ "$FAIL" -ne 0 ]; then
	exit 1
fi

echo "ci: gdUnit summary OK ($SUITE_COUNT suites, $CASE_COUNT cases, zero errors/failures/skips)"
