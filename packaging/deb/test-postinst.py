"""Exercise postinst without installing a package or modifying system directories."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class PostinstTests(unittest.TestCase):
    def run_postinst(self, action, fail_updates=False):
        script = Path(__file__).with_name("postinst")
        with tempfile.TemporaryDirectory(prefix="expressa-postinst-") as temporary:
            root = Path(temporary)
            commands = root / "bin"
            commands.mkdir()
            # Record any attempt to inspect accounts or install user files too.
            for name in (
                "getent", "su", "expressa", "cp",
                "update-desktop-database", "update-mime-database", "gtk-update-icon-cache",
            ):
                mock = commands / name
                mock.write_text(
                    '#!/bin/sh\nprintf "%s\\n" "${0##*/} $*" >> "$TEST_LOG"\n'
                    'exit "$TEST_EXIT"\n'
                )
                mock.chmod(0o755)
            log = root / "calls"
            env = dict(os.environ)
            env.update(
                PATH=f"{commands}:/usr/bin:/bin",
                TEST_LOG=str(log),
                TEST_EXIT="1" if fail_updates else "0",
                SUDO_USER="aluno",
                PKEXEC_UID="1000",
            )
            result = subprocess.run(
                ["/bin/sh", str(script), action], env=env, capture_output=True, text=True
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, "")
            return log.read_text().splitlines() if log.exists() else []

    def test_configure_only_updates_desktop_metadata(self):
        self.assertEqual(self.run_postinst("configure"), [
            "update-desktop-database -q /usr/share/applications",
            "update-mime-database /usr/share/mime",
            "gtk-update-icon-cache -q /usr/share/icons/hicolor",
        ])

    def test_metadata_update_failure_does_not_fail_installation(self):
        self.assertEqual(len(self.run_postinst("configure", fail_updates=True)), 3)

    def test_other_actions_do_nothing(self):
        self.assertEqual(self.run_postinst("abort-upgrade"), [])


if __name__ == "__main__":
    unittest.main()
