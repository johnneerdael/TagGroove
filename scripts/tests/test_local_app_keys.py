import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "local-app-keys.py"
spec = importlib.util.spec_from_file_location("local_app_keys", SCRIPT)
keys = importlib.util.module_from_spec(spec)
spec.loader.exec_module(keys)


def write(directory, name, text):
    (Path(directory) / name).write_text(text, encoding="utf-8")


class LocalAppKeys(unittest.TestCase):
    def test_each_credential_file_format_is_read_without_the_surrounding_text(self):
        with tempfile.TemporaryDirectory() as directory:
            write(directory, ".acoustid", "acoustid-key-1\n")
            write(directory, ".fanart", "\nfanart0123456789abcdef\n")
            write(directory, ".discogs", "Consumer Key\tdiscogskey12345678\nConsumer Secret\tdiscogssecret1234567890\nRequest Token URL\thttps://api.discogs.com/oauth/request_token\n")
            write(directory, ".lastfm", "Application name\tTagGroove\nAPI key\tlastfmkey0123456789\nShared secret\tlastfmsecret0123456789\nRegistered to\tsomeone\n")
            found = keys.load(directory)
        self.assertEqual(found, {
            "OPENTAG_ACOUSTID_API_KEY": "acoustid-key-1",
            "OPENTAG_FANARTTV_API_KEY": "fanart0123456789abcdef",
            "OPENTAG_DISCOGS_CONSUMER_KEY": "discogskey12345678",
            "OPENTAG_DISCOGS_CONSUMER_SECRET": "discogssecret1234567890",
            "OPENTAG_LASTFM_API_KEY": "lastfmkey0123456789",
            "OPENTAG_LASTFM_SHARED_SECRET": "lastfmsecret0123456789",
        })

    def test_missing_or_malformed_files_leave_their_variables_unset(self):
        with tempfile.TemporaryDirectory() as directory:
            write(directory, ".acoustid", "not a key\n")
            write(directory, ".lastfm", "API key\tlastfmkey0123456789\n")
            self.assertEqual(keys.load(directory), {"OPENTAG_LASTFM_API_KEY": "lastfmkey0123456789"})

    def test_check_names_variables_but_never_prints_a_value(self):
        with tempfile.TemporaryDirectory() as directory:
            write(directory, ".fanart", "fanart0123456789abcdef\n")
            done = subprocess.run([sys.executable, str(SCRIPT), "--dir", directory, "--check"], capture_output=True, text=True, check=True)
        self.assertIn("OPENTAG_FANARTTV_API_KEY: set", done.stdout)
        self.assertIn("OPENTAG_LASTFM_API_KEY: missing", done.stdout)
        self.assertNotIn("fanart0123456789abcdef", done.stdout + done.stderr)

    def test_a_command_receives_the_variables_in_its_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            write(directory, ".acoustid", "acoustid-key-1\n")
            env = {k: v for k, v in os.environ.items() if not k.startswith("OPENTAG_")}
            done = subprocess.run(
                [sys.executable, str(SCRIPT), "--dir", directory, "--", sys.executable, "-c",
                 "import os; print(os.environ['OPENTAG_ACOUSTID_API_KEY'])"],
                capture_output=True, text=True, check=True, env=env)
        self.assertEqual(done.stdout.strip(), "acoustid-key-1")


if __name__ == "__main__":
    unittest.main()
