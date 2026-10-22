#!/usr/bin/env python3
"""Retry stapling and validate tickets without exposing Apple tool output."""
import os
import platform
import subprocess
import sys
import tempfile
import time

REAL_XCRUN = '/usr/bin/xcrun'
# sysexits: invalid/service ticket data, unexpected service response,
# unavailable service, and temporary failure. Revocation, missing input and
# usage errors refuse immediately; every exhausted retry remains a failure.
RETRYABLE = {65, 68, 69, 75}


def run_operation(arguments):
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        try:
            result = subprocess.run([REAL_XCRUN, *arguments], stdin=subprocess.DEVNULL,
                                    stdout=output, stderr=errors, timeout=120)
        except subprocess.TimeoutExpired:
            return 75
        if any(stream.seek(0, os.SEEK_END) > 1024*1024 for stream in (output, errors)):
            return 74
    return result.returncode


def process(arguments, *, run=run_operation, pause=time.sleep):
    if len(arguments) < 3 or arguments[:2] not in [['stapler', 'staple'], ['stapler', 'validate']]:
        return 64
    for attempt in range(1, 6):
        status = run(arguments)
        if status == 0 and arguments[1] == 'staple':
            status = run(['stapler', 'validate', arguments[-1]])
        if status == 0:
            return 0
        print('macOS ticket '+arguments[1]+' attempt '+str(attempt)+' failed (exit '+str(status)+')', flush=True)
        if status not in RETRYABLE or attempt == 5:
            return status
        pause(15*2**(attempt-1))
    return 75


def main(arguments=None):
    arguments = list(sys.argv[1:] if arguments is None else arguments)
    if platform.system() != 'Darwin':
        raise SystemExit('Checked stapler requires macOS')
    if arguments[:2] in [['stapler', 'staple'], ['stapler', 'validate']]:
        raise SystemExit(process(arguments))
    # SDK/compiler discovery and notarization retain the exact system behavior.
    os.execv(REAL_XCRUN, [REAL_XCRUN, *arguments])


if __name__ == '__main__':
    try:main()
    except (OSError, ValueError, subprocess.SubprocessError):
        raise SystemExit('Checked Apple tool operation failed') from None
