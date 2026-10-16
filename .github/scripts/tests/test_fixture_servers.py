"""Exercise the real native-suite HTTP handlers together, without opening an app."""
import http.client
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import time
import unittest
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[3]


class FixtureServerTests(unittest.TestCase):
    def test_gnudb_and_builtin_provider_handlers_coexist(self):
        with tempfile.TemporaryDirectory() as directory:
            site = Path(directory)
            gnudb = (ROOT / "tests/e2e/gnudb-fixtures.sh").read_text()
            provider = (ROOT / "tests/e2e/sources-4-fixtures.sh").read_text()
            cddb_code = gnudb.split("<<'PY'\n", 1)[1].split("\nPY\n", 1)[0]
            provider_code = provider.split("serve='", 1)[1].split("'\n# The server", 1)[0]
            answer = re.search(r'cat >"\$site/itunes.apple.com_search" <<\x27JSON\x27\n(.*?)\nJSON', provider, re.S)[1]
            (site / "itunes.apple.com_search").write_text(answer)
            servers = []

            def request(port, path):
                connection = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
                try:
                    connection.request("GET", path)
                    response = connection.getresponse()
                    return response.status, response.read()
                finally:
                    connection.close()

            def start(code, argument, ready):
                port = int(re.search(r'ThreadingHTTPServer\(\("127.0.0.1", (\d+)\)', code)[1])
                process = subprocess.Popen([sys.executable, "-I", "-c", code, argument],
                                           stdout=subprocess.PIPE, stderr=subprocess.PIPE)
                servers.append(process)
                for _ in range(50):
                    if process.poll() is not None:
                        self.fail("Fixture server exited: " + process.communicate()[1].decode())
                    try:
                        if request(port, ready)[0] == 200:
                            return port
                    except OSError:
                        pass
                    time.sleep(0.05)
                self.fail("Fixture HTTP readiness timed out")

            try:
                cddb_port = start(cddb_code, "150", "/ready")
                provider_port = start(provider_code, str(site), "/itunes.apple.com/search")
                query = {"hello": "john neerdael.nl OpenTagAudioTagEditor v0.2.0", "proto": "6",
                         "cmd": "cddb query 07000602 2 150 375 8"}
                status, body = request(cddb_port, "/~cddb/cddb.cgi?" + urlencode(query))
                self.assertEqual(status, 200)
                self.assertIn(b"folk 1a2b3c4d", body)
                query["cmd"] = "cddb read folk 1a2b3c4d"
                self.assertIn(b"DTITLE=Fixture Artist / Layout Album",
                              request(cddb_port, "/~cddb/cddb.cgi?" + urlencode(query))[1])
                status, body = request(provider_port, "/itunes.apple.com/search")
                self.assertEqual(status, 200)
                self.assertEqual(json.loads(body)["results"][0]["collectionId"], 900001)
                self.assertEqual(request(cddb_port, "/ready")[0], 200)
                self.assertEqual(request(provider_port, "/ready")[0], 404)
                self.assertEqual(request(cddb_port, "/itunes.apple.com/search")[0], 404)
            finally:
                for process in servers:
                    if process.poll() is None:
                        process.terminate()
                    process.communicate(timeout=5)
