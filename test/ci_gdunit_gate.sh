#!/bin/sh
# Behavioral self-test for ci/run_gdunit.sh.
#
# gdUnit4 can exit zero after discovering only the parseable subset of a test
# tree.  The production gate therefore has two independent witnesses: a
# source census of suites/test functions and the runner's three terminal
# summary records.  These fixtures hand-derive two suites and three cases,
# then prove that missing, partial, contradictory, skipped, or failed output
# cannot masquerade as a complete run.
#
# Pure POSIX sh; no Godot, network, or repository mutation.
set -eu
DIR="$(cd "$(dirname "$0")/.." && pwd)"
GATE="$DIR/ci/run_gdunit.sh"
FAIL=0

ok() { echo "gdunit-gate: OK   $1"; }
bad() { echo "gdunit-gate: FAIL $1"; FAIL=1; }

FIXTURE="$(mktemp -d)"
trap 'rm -rf "$FIXTURE"' EXIT INT TERM HUP
mkdir -p "$FIXTURE/game/tests"

cat >"$FIXTURE/game/tests/alpha_test.gd" <<'GDUNIT_SUITE'
extends GdUnitTestSuite

func test_alpha_breaks_if_the_first_case_is_lost() -> void:
	pass

func test_alpha_breaks_if_the_second_case_is_lost() -> void:
	pass
GDUNIT_SUITE

cat >"$FIXTURE/game/tests/beta_test.gd" <<'GDUNIT_SUITE'
extends GdUnitTestSuite

func test_beta_breaks_if_its_suite_is_lost() -> void:
	pass
GDUNIT_SUITE

cat >"$FIXTURE/fake-gdunit" <<'FAKE_RUNNER'
#!/bin/sh
printf '%s\n' "${FAKE_GDUNIT_OUTPUT:-}"
exit "${FAKE_GDUNIT_STATUS:-0}"
FAKE_RUNNER
chmod +x "$FIXTURE/fake-gdunit"

RESULT="$FIXTURE/result.log"

run_gate() { # run_gate <runner-output> <runner-status>
	FAKE_GDUNIT_OUTPUT="$1" FAKE_GDUNIT_STATUS="$2" \
		"$GATE" "$FIXTURE/game" "$FIXTURE/fake-gdunit" >"$RESULT" 2>&1
}

expect_pass() { # expect_pass <label> <runner-output>
	if run_gate "$2" 0; then
		ok "$1"
	else
		bad "$1 (a complete 2-suite/3-case run was rejected)"
		sed -n '1,20p' "$RESULT"
	fi
}

expect_reject() { # expect_reject <label> <runner-output> <runner-status> <diagnostic-fragment>
	if run_gate "$2" "$3"; then
		bad "$1 (the incomplete run was accepted)"
		return
	fi
	if grep -qF "$4" "$RESULT"; then
		ok "$1"
	else
		bad "$1 (rejected without the expected '$4' diagnostic)"
		sed -n '1,20p' "$RESULT"
	fi
}

GOOD_OUTPUT='Overall Summary: 3 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (2/2)
Executed test cases : (3/3)'

expect_pass "the source-derived 2-suite/3-case summary passes" "$GOOD_OUTPUT"

# Real gdUnit4 wraps each field in ANSI CSI colour sequences.  A parser that
# checks the pretty-looking text without normalising those bytes rejects every
# real run despite accepting the plain fixture above.
ESC="$(printf '\033')"
ANSI_OUTPUT="${ESC}[38;2;30;144;255mOverall Summary:${ESC}[0m${ESC}[38;2;255;255;255m 3 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |${ESC}[0m
${ESC}[38;2;233;150;122mExecuted test suites: (2/2)${ESC}[0m
${ESC}[38;2;233;150;122mExecuted test cases : (3/3)${ESC}[0m"
expect_pass "real-style ANSI colour does not hide an exact summary" "$ANSI_OUTPUT"

expect_reject \
	"a zero exit with no terminal summary is rejected" \
	"all individual tests looked green" 0 "missing Overall Summary"

expect_reject \
	"a parse-lost suite is rejected against the source census" \
	'Overall Summary: 2 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (1/1)
Executed test cases : (2/2)' \
	0 "expected exactly: Overall Summary: 3 test cases"

expect_reject \
	"a parse-lost case is rejected against the source census" \
	'Overall Summary: 2 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (2/2)
Executed test cases : (2/2)' \
	0 "expected exactly: Overall Summary: 3 test cases"

expect_reject \
	"a wrong suite ratio is rejected even when both case records look complete" \
	'Overall Summary: 3 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (1/1)
Executed test cases : (3/3)' \
	0 "expected exactly: Executed test suites: (2/2)"

expect_reject \
	"a wrong case ratio is rejected even when the overall record looks complete" \
	'Overall Summary: 3 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (2/2)
Executed test cases : (2/2)' \
	0 "expected exactly: Executed test cases : (3/3)"

expect_reject \
	"errors cannot hide behind complete executed ratios" \
	'Overall Summary: 3 test cases | 1 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (2/2)
Executed test cases : (3/3)' \
	0 "expected exactly: Overall Summary: 3 test cases | 0 errors"

expect_reject \
	"failures cannot hide behind complete executed ratios" \
	'Overall Summary: 3 test cases | 0 errors | 1 failures | 0 flaky | 0 skipped | 0 orphans |
Executed test suites: (2/2)
Executed test cases : (3/3)' \
	0 "expected exactly: Overall Summary: 3 test cases | 0 errors | 0 failures"

expect_reject \
	"skipped cases cannot satisfy the exact executed-case record" \
	'Overall Summary: 3 test cases | 0 errors | 0 failures | 0 flaky | 1 skipped | 0 orphans |
Executed test suites: (2/2)
Executed test cases : (2/3), 1 skipped' \
	0 "expected exactly: Overall Summary: 3 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped"

expect_reject \
	"a runner failure remains a failure even with forged green text" \
	"$GOOD_OUTPUT" 7 "gdUnit4 exited with status 7"

expect_reject \
	"contradictory duplicate summaries are rejected" \
	"$GOOD_OUTPUT
Overall Summary: 2 test cases | 0 errors | 0 failures | 0 flaky | 0 skipped | 0 orphans |" \
	0 "found 2 Overall Summary records"

# A source file with the suite naming convention but no test declaration is
# ambiguous to a line census and dangerous to ignore.  Reject it before the
# fake runner is even consulted.
cat >"$FIXTURE/game/tests/empty_test.gd" <<'GDUNIT_SUITE'
extends GdUnitTestSuite
GDUNIT_SUITE
expect_reject \
	"an empty source suite fails loudly instead of shrinking the count" \
	"$GOOD_OUTPUT" 0 "declares no test_ functions"
rm "$FIXTURE/game/tests/empty_test.gd"

# Keep the behavioral test and the production invocation inseparable from the
# pipeline.  These are literal call sites, not duplicate implementations.
SELF_TEST_CALL="\"\$DIR/test/ci_gdunit_gate.sh\""
RUNNER_CALL="\"\$DIR/ci/run_gdunit.sh\""
if grep -qF "$SELF_TEST_CALL" "$DIR/ci/pipeline.sh"; then
	ok "the cheap behavioral self-test is wired into the pipeline"
else
	bad "ci/pipeline.sh does not run test/ci_gdunit_gate.sh"
fi
if grep -qF "$RUNNER_CALL" "$DIR/ci/pipeline.sh"; then
	ok "the real gdUnit stage is wired through the strict gate"
else
	bad "ci/pipeline.sh bypasses ci/run_gdunit.sh"
fi

exit "$FAIL"
