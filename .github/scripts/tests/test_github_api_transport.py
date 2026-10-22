"""Exercise trusted metadata reads without contacting GitHub or repeating writes."""
from contextlib import redirect_stderr
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch


def module():
    script=Path(__file__).resolve().parents[1]/'codex_review.py'
    spec=importlib.util.spec_from_file_location('metadata_transport',script)
    result=importlib.util.module_from_spec(spec);spec.loader.exec_module(result)
    return result


class GitHubAPITransport(unittest.TestCase):
    def test_read_recovers_without_exposing_private_error_output(self):
        m=module();api=m.GitHub('example/project');errors=io.StringIO()
        failure=subprocess.CalledProcessError(1,['gh','api'],stderr='private-credential HTTP 503')
        success=subprocess.CompletedProcess([],0,stdout='{"ready":true}')
        with patch.object(m.subprocess,'run',side_effect=[failure,failure,success]) as run,patch('time.sleep') as pause,redirect_stderr(errors):
            try:result=api.get('commits/fixture')
            except subprocess.CalledProcessError:self.fail('A temporary metadata read must receive bounded retries')
        self.assertEqual(result,{'ready':True});self.assertEqual(run.call_count,3)
        self.assertEqual([call.args[0] for call in pause.call_args_list],[5,10])
        self.assertTrue(all(call.kwargs['timeout']==120 for call in run.call_args_list))
        self.assertIn('HTTP 503',errors.getvalue());self.assertNotIn('private-credential',errors.getvalue())

    def test_pagination_discards_partial_output_before_retrying(self):
        m=module();api=m.GitHub('example/project')
        failure=subprocess.CalledProcessError(1,['gh','api'],output='[[{"id":99}]]',stderr='HTTP 502')
        success=subprocess.CompletedProcess([],0,stdout='[[{"id":1}],[{"id":2}]]')
        with patch.object(m.subprocess,'run',side_effect=[failure,success]) as run,patch('time.sleep'),redirect_stderr(io.StringIO()):
            try:result=api.pages('pulls?per_page=100')
            except subprocess.CalledProcessError:self.fail('Retry the complete paginated read after a transport failure')
        self.assertEqual(result,[{'id':1},{'id':2}])
        self.assertEqual(run.call_count,2)
        for call in run.call_args_list:self.assertEqual(call.args[0][-2:],['--paginate','--slurp'])

    def test_exhausted_reads_preserve_failure_and_only_report_status(self):
        m=module();api=m.GitHub('example/project');errors=io.StringIO()
        failure=subprocess.CalledProcessError(1,['gh','api'],stderr='private-body HTTP 403')
        with patch.object(m.subprocess,'run',side_effect=failure) as run,patch('time.sleep') as pause,redirect_stderr(errors):
            with self.assertRaises(subprocess.CalledProcessError) as stopped:api.get('commits/fixture')
        self.assertIs(stopped.exception,failure);self.assertEqual(run.call_count,3)
        self.assertEqual([call.args[0] for call in pause.call_args_list],[5,10])
        self.assertIn('HTTP 403',errors.getvalue());self.assertNotIn('private-body',errors.getvalue())

    def test_timed_out_read_can_recover(self):
        m=module();api=m.GitHub('example/project');errors=io.StringIO()
        failure=subprocess.TimeoutExpired('private-command',120)
        success=subprocess.CompletedProcess([],0,stdout='{"ready":true}')
        with patch.object(m.subprocess,'run',side_effect=[failure,success]) as run,patch('time.sleep'),redirect_stderr(errors):
            try:result=api.get('commits/fixture')
            except subprocess.TimeoutExpired:self.fail('A timed-out read must receive bounded retries')
        self.assertEqual(result,{'ready':True});self.assertEqual(run.call_count,2)
        self.assertNotIn('private-command',errors.getvalue())

    def test_repeated_read_timeouts_still_refuse_after_three_attempts(self):
        m=module();api=m.GitHub('example/project');failure=subprocess.TimeoutExpired('private-command',120)
        with patch.object(m.subprocess,'run',side_effect=failure) as run,patch('time.sleep'),redirect_stderr(io.StringIO()):
            with self.assertRaises(subprocess.TimeoutExpired):api.get('commits/fixture')
        self.assertEqual(run.call_count,3)

    def test_writes_are_never_repeated_or_logged(self):
        m=module();api=m.GitHub('example/project');errors=io.StringIO()
        failure=subprocess.CalledProcessError(1,['gh','api'],stderr='private-write-response HTTP 503')
        with patch.object(m.subprocess,'run',side_effect=failure) as run,patch('time.sleep') as pause,redirect_stderr(errors):
            with self.assertRaises(subprocess.CalledProcessError):api.request('repos/example/project/statuses/fixture',{'body':'private-input'})
        self.assertEqual(run.call_count,1);pause.assert_not_called();self.assertEqual(errors.getvalue(),'')
        self.assertEqual(json.loads(run.call_args.kwargs['input']),{'body':'private-input'})

    def test_invalid_successful_metadata_is_not_retried_or_substituted(self):
        m=module();api=m.GitHub('example/project')
        with patch.object(m.subprocess,'run',return_value=subprocess.CompletedProcess([],0,stdout='not JSON')) as run,patch('time.sleep') as pause:
            with self.assertRaises(json.JSONDecodeError):api.get('commits/fixture')
        self.assertEqual(run.call_count,1);pause.assert_not_called()
