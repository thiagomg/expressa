"""Exercise postinst without installing a package or modifying system directories."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class PostinstTests(unittest.TestCase):
    def run_postinst(self, action, fail_user=""):
        script = Path(__file__).with_name("postinst")
        with tempfile.TemporaryDirectory(prefix="expressa-postinst-") as temporary:
            root = Path(temporary)
            commands = root / "bin"
            commands.mkdir()
            home = root / "aluno"
            home.mkdir()
            uid_min = 1000
            for line in Path("/etc/login.defs").read_text().splitlines():
                fields = line.split()
                if fields and fields[0] == "UID_MIN":
                    uid_min = int(fields[1])
            passwd = root / "passwd"
            passwd.write_text(
                f"root:x:0:0:root:{home}:/bin/sh\n"
                f"servico:x:{uid_min - 1}:1::{home}:/bin/sh\n"
                f"aluno:x:{uid_min}:1000::{home}:/bin/sh\n"
                f"outra:x:{uid_min + 1}:1000::{home}:/bin/sh\n"
                f"ausente:x:{uid_min + 2}:1000::{root / 'ausente'}:/bin/sh\n"
            )
            mocks = {
                "getent": '#!/bin/sh\ncat "$TEST_PASSWD"\n',
                "su": '#!/bin/sh\nprintf "%s\\n" "$*" >> "$TEST_LOG"\n[ "$4" != "$TEST_FAIL_USER" ]\n',
                "update-desktop-database": "#!/bin/sh\nexit 0\n",
                "update-mime-database": "#!/bin/sh\nexit 0\n",
                "gtk-update-icon-cache": "#!/bin/sh\nexit 0\n",
            }
            for name, content in mocks.items():
                mock = commands / name
                mock.write_text(content)
                mock.chmod(0o755)
            log = root / "calls"
            env = dict(os.environ)
            env.pop("SUDO_USER", None)
            env.update(
                PATH=f"{commands}:/usr/bin:/bin",
                TEST_PASSWD=str(passwd),
                TEST_LOG=str(log),
                TEST_FAIL_USER=fail_user,
            )
            result = subprocess.run(
                ["/bin/sh", str(script), action], env=env, capture_output=True, text=True
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            return log.read_text().splitlines() if log.exists() else [], result.stderr

    def test_configure_without_sudo_user(self):
        calls, errors = self.run_postinst("configure")
        self.assertEqual(calls, [
            "-s /bin/sh - aluno -c /usr/bin/expressa --instalar-exemplos",
            "-s /bin/sh - outra -c /usr/bin/expressa --instalar-exemplos",
        ])
        self.assertEqual(errors, "")

    def test_failure_is_visible_and_other_students_still_receive_examples(self):
        calls, errors = self.run_postinst("configure", fail_user="aluno")
        self.assertEqual(len(calls), 2)
        self.assertIn("não foi possível instalar exemplos para aluno", errors)

    def test_other_actions_do_not_copy(self):
        calls, errors = self.run_postinst("abort-upgrade")
        self.assertEqual(calls, [])
        self.assertEqual(errors, "")


if __name__ == "__main__":
    unittest.main()
