"""Runner mirror configuration preserves package trust and unrelated sources."""
import importlib.util
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[2] / "actions/ubuntu-mirrors/configure.py"


class UbuntuMirrors(unittest.TestCase):
    def configure(self, root, mirrors, architecture="amd64"):
        self.assertTrue(SCRIPT.exists(), "Runner mirror configuration is missing")
        spec = importlib.util.spec_from_file_location("ubuntu_mirrors", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        module.configure(root, mirrors, architecture)

    def test_geographic_list_with_archive_fallback_preserves_signed_by_and_security(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            sources = root / "sources.list.d"
            sources.mkdir()
            source = sources / "ubuntu.sources"
            original = ("Types: deb\nURIs: http://azure.archive.ubuntu.com/ubuntu/\n"
                        "Suites: noble noble-updates\nComponents: main universe\n"
                        "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n\n"
                        "Types: deb\nURIs: http://security.ubuntu.com/ubuntu/\nSuites: noble-security\n")
            source.write_text(original)
            unrelated = sources / "vendor.list"
            unrelated.write_text("deb https://packages.example.org/ubuntu noble main\n")
            self.configure(root, "https://mirror.example.org/ubuntu/\n")
            self.assertEqual(source.read_text(), original.replace(
                "http://azure.archive.ubuntu.com/ubuntu/",
                "mirror+file:" + str(root / "taggroove-mirrors.txt")))
            self.assertEqual(unrelated.read_text(), "deb https://packages.example.org/ubuntu noble main\n")
            mirrorlist = (root / "taggroove-mirrors.txt").read_text()
            self.assertIn("https://mirror.example.org/ubuntu/\tpriority:1", mirrorlist)
            self.assertIn("http://archive.ubuntu.com/ubuntu/\tpriority:10", mirrorlist)
            config = (root / "apt.conf.d/80taggroove-network").read_text()
            self.assertIn('Acquire::http::Timeout "20";', config)
            self.assertIn('Acquire::https::Timeout "20";', config)
            self.assertIn('Acquire::Retries "2";', config)
            self.assertNotIn("AllowInsecure", config)

    def test_empty_or_invalid_list_uses_archive_and_leaves_arm_ports_alone(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "sources.list"
            original = ("deb http://ports.ubuntu.com/ubuntu-ports noble main\n"
                        "deb https://archive.ubuntu.com/ubuntu noble main\n")
            source.write_text(original)
            self.configure(root, "file:/tmp/local\nhttp://user:pass@example.org/ubuntu/\nhttp://[invalid\n<html>oops</html>\n")
            self.assertIn(original.splitlines()[0], source.read_text())
            self.assertIn("mirror+file:", source.read_text().splitlines()[1])
            self.assertEqual((root / "taggroove-mirrors.txt").read_text(),
                             "http://archive.ubuntu.com/ubuntu/\tpriority:10\n")

    def test_repeated_configuration_is_stable(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "sources.list"
            source.write_text("deb http://azure.archive.ubuntu.com/ubuntu noble main\n")
            self.configure(root, "https://mirror.example.org/ubuntu/\n")
            first = source.read_text()
            self.configure(root, "https://mirror.example.org/ubuntu/\n")
            self.assertEqual(source.read_text(), first)

    def test_github_runner_existing_mirror_list_is_replaced(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            sources = root / "sources.list.d"
            sources.mkdir()
            source = sources / "ubuntu.sources"
            source.write_text("Types: deb\nURIs: mirror+file:/etc/apt/apt-mirrors.txt\n"
                              "Suites: noble noble-updates noble-security\n"
                              "Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg\n")
            self.configure(root, "https://mirror.example.org/ubuntu/\n")
            self.assertIn("URIs: mirror+file:" + str(root / "taggroove-mirrors.txt"), source.read_text())
            self.assertNotIn("/etc/apt/apt-mirrors.txt", source.read_text())
            self.assertIn("Suites: noble noble-updates noble-security", source.read_text())
            self.assertIn("Signed-By: /usr/share/keyrings/ubuntu-archive-keyring.gpg", source.read_text())

    def test_arm_runner_keeps_its_architecture_specific_mirror_selection(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            source = root / "sources.list"
            original = "deb mirror+file:/etc/apt/apt-mirrors.txt noble main\n"
            source.write_text(original)
            self.configure(root, "https://mirror.example.org/ubuntu/\n", "arm64")
            self.assertEqual(source.read_text(), original)
            self.assertTrue((root / "apt.conf.d/80taggroove-network").exists())


if __name__ == "__main__":
    unittest.main()
