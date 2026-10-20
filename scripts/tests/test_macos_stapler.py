"""Check ticket retry ordering without running Apple tools or building an app."""
from contextlib import redirect_stderr, redirect_stdout
import importlib.util
import io
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch


SCRIPT = Path(__file__).resolve().parents[1]/'macos-stapler.py'


class MacOSStapler(unittest.TestCase):
    def module(self):
        self.assertTrue(SCRIPT.is_file(), 'Missing checked pre-DMG stapler wrapper')
        spec = importlib.util.spec_from_file_location('macos_stapler', SCRIPT)
        module = importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        return module

    def test_staple_retries_service_failure_and_requires_validation_before_returning_success(self):
        m=self.module();calls=[];delays=[];statuses=iter([68,0,68,0,0])
        def run(arguments):calls.append(arguments);return next(statuses)
        args=['stapler','staple','-v','private-fixture.app']
        output=io.StringIO()
        with redirect_stdout(output):self.assertEqual(m.process(args,run=run,pause=delays.append),0)
        self.assertEqual(calls,[args,args,['stapler','validate',args[-1]],args,['stapler','validate',args[-1]]])
        self.assertEqual(delays,[15,30])
        self.assertNotIn('private-fixture',output.getvalue())

    def test_permanent_or_exhausted_failure_never_returns_success(self):
        m=self.module()
        for status,attempts in [(65,3),(68,3),(77,1),(64,1),(66,1)]:
            calls=[];delays=[]
            def run(arguments):calls.append(arguments);return status
            with self.subTest(status=status),redirect_stdout(io.StringIO()):
                self.assertEqual(m.process(['stapler','staple','private-fixture.app'],run=run,pause=delays.append),status)
            self.assertEqual(len(calls),attempts)
            self.assertEqual(delays,[15,30] if attempts==3 else [])

    def test_readonly_validation_retries_without_stapling_or_modifying_the_item(self):
        m=self.module();calls=[];delays=[];statuses=iter([68,0])
        def run(arguments):calls.append(arguments);return next(statuses)
        args=['stapler','validate','readonly-fixture.app']
        with redirect_stdout(io.StringIO()):self.assertEqual(m.process(args,run=run,pause=delays.append),0)
        self.assertEqual(calls,[args,args]);self.assertEqual(delays,[15])

    def test_other_xcrun_operations_pass_through_to_the_absolute_apple_tool(self):
        m=self.module();args=['--sdk','macosx','--show-sdk-path']
        with patch.object(m.platform,'system',return_value='Darwin'),patch.object(m.os,'execv') as execute:
            m.main(args)
        execute.assert_called_once_with('/usr/bin/xcrun',['/usr/bin/xcrun',*args])

    def test_successful_staple_with_permanently_invalid_ticket_still_refuses(self):
        m=self.module();calls=[];delays=[]
        def run(args):calls.append(args);return 0 if args[1]=='staple' else 65
        with redirect_stdout(io.StringIO()):
            self.assertEqual(m.process(['stapler','staple','fixture.app'],run=run,pause=delays.append),65)
        self.assertEqual(len(calls),6);self.assertEqual(delays,[15,30])

    def test_real_command_capture_never_echoes_private_diagnostics_and_refuses_oversized_output(self):
        m=self.module();output=io.StringIO();errors=io.StringIO()
        with patch.object(m,'REAL_XCRUN',sys.executable),redirect_stdout(output),redirect_stderr(errors):
            self.assertEqual(m.run_operation(['-c','import sys;print("private-key-path");print("private-tool-error",file=sys.stderr);sys.exit(68)']),68)
            self.assertEqual(m.run_operation(['-c','print("x"*(1024*1024+1))']),74)
        self.assertEqual(output.getvalue(),'');self.assertEqual(errors.getvalue(),'')

    def test_command_timeout_is_a_bounded_retryable_failure_without_private_exception_text(self):
        m=self.module();output=io.StringIO()
        with patch.object(m.subprocess,'run',side_effect=subprocess.TimeoutExpired('private-key-path',120)),redirect_stdout(output):
            self.assertEqual(m.run_operation(['stapler','validate','fixture.app']),75)
        self.assertEqual(output.getvalue(),'')
