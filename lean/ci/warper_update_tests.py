import hashlib
import io
import os
import re
import select
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
import zipfile
from pathlib import Path

SCRIPT = Path(__file__).resolve().parents[1] / "warper-update"
TAG = "lean-v0.10.0"


def write_executable(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(0o755)


class UpdateTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self.addCleanup(subprocess.run, ["rm", "-rf", str(self.root)])
        self.release = self.root / "release"
        self.shims = self.root / "shims"
        self.shims.mkdir()

    def publish(self, asset, files, *, tag=TAG, checksum=True, corrupt=False):
        (self.release / tag).mkdir(parents=True, exist_ok=True)
        (self.release / "latest.json").write_text(f'{{\n  "tag_name": "{tag}"\n}}\n')
        buffer = io.BytesIO()
        if asset.endswith(".zip"):
            with zipfile.ZipFile(buffer, "w") as archive:
                for name, data in files.items():
                    archive.writestr(name, data)
        else:
            with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
                for name, data in files.items():
                    info = tarfile.TarInfo(name)
                    info.size = len(data)
                    info.mode = 0o755
                    archive.addfile(info, io.BytesIO(data.encode()))
        payload = buffer.getvalue()
        (self.release / tag / asset).write_bytes(payload)
        if checksum:
            digest = "0" * 64 if corrupt else hashlib.sha256(payload).hexdigest()
            (self.release / tag / "SHA256SUMS").write_text(f"{digest}  {asset}\n")

    def shim(self, name, body):
        write_executable(self.shims / name, "#!/bin/sh\n" + body)

    def environment(self, **env):
        return {
            "PATH": f"{self.shims}:{os.environ['PATH']}",
            "HOME": str(self.root / "home"),
            "WARPER_UPDATE_API": f"file://{self.release}/latest.json",
            "WARPER_UPDATE_DOWNLOAD": f"file://{self.release}",
            "TMPDIR": str(self.root),
            **env,
        }

    def run_script(self, answers, script=SCRIPT, **env):
        return subprocess.run(
            ["sh", str(script)],
            input=answers,
            text=True,
            capture_output=True,
            env=self.environment(**env),
            timeout=60,
        )

    def start_script(self, script=SCRIPT, **env):
        process = subprocess.Popen(
            ["sh", str(script)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            env=self.environment(**env),
        )
        # Cleanups run last-in first-out: kill, then reap, then close the pipes.
        self.addCleanup(process.stdout.close)
        self.addCleanup(process.stdin.close)
        self.addCleanup(process.wait)
        self.addCleanup(process.kill)
        return process

    def answer(self, process, text):
        process.stdin.write(text.encode())
        process.stdin.flush()

    def read_until(self, process, text, timeout=30):
        seen = ""
        deadline = time.time() + timeout
        while text not in seen:
            remaining = deadline - time.time()
            self.assertGreater(remaining, 0, f"timed out waiting for {text!r}; got {seen!r}")
            if select.select([process.stdout], [], [], remaining)[0]:
                chunk = os.read(process.stdout.fileno(), 4096)
                self.assertTrue(chunk, f"the script ended before {text!r}; got {seen!r}")
                seen += chunk.decode()
        return seen

    def staging_dirs(self, directory):
        return [p for p in directory.iterdir() if p.name.startswith(".warper-update.")]

    def start_warper(self):
        running = subprocess.Popen(["sleep", "300"])
        self.addCleanup(running.kill)
        return running

    def wait_for(self, path):
        deadline = time.time() + 20
        while not path.exists() and time.time() < deadline:
            time.sleep(0.1)
        return path.exists()

    def helper_pids(self, running):
        """The detached helpers waiting for this process to exit."""
        processes = subprocess.run(
            ["ps", "-axo", "pid=,command="], text=True, capture_output=True, check=True
        ).stdout.splitlines()
        return [
            int(line.split()[0])
            for line in processes
            if "kill -0" in line and str(running.pid) in line.split(None, 1)[1]
        ]

    def wait_until_gone(self, path):
        deadline = time.time() + 20
        while path.exists() and time.time() < deadline:
            time.sleep(0.1)
        return not path.exists()


class LinuxUpdateTests(UpdateTests):
    def setUp(self):
        super().setUp()
        self.shim("uname", 'case "$1" in -s) echo Linux ;; -m) echo aarch64 ;; esac\n')
        self.prefix = self.root / "prefix"
        self.app = self.prefix / "lib" / "warp-lean"
        write_executable(self.app / "warp-oss", "#!/bin/sh\necho old\n")
        shutil.copy2(SCRIPT, self.app / "warper-update")
        (self.app / "resources").mkdir()
        (self.app / "resources" / "old.json").write_text("{}")
        self.package = {
            "warper-linux-arm64/warper": "#!/bin/sh\necho new > \"$MARKER\"\n",
            "warper-linux-arm64/warper-update": "#!/bin/sh\n# new updater\n",
            "warper-linux-arm64/resources/schema.json": "{}",
        }

    def install_version(self, version):
        (self.app / "VERSION").write_text(version + "\n")

    def run_linux(self, answers, **env):
        return self.run_script(answers, script=self.app / "warper-update", **env)

    def assert_old_install_intact(self):
        self.assertIn("old", (self.app / "warp-oss").read_text())
        self.assertEqual((self.app / "VERSION").read_text(), "0.9.0\n")
        self.assertEqual((self.app / "warper-update").read_text(), SCRIPT.read_text())
        self.assertTrue((self.app / "resources" / "old.json").exists())
        self.assertFalse((self.app / "warp-oss.previous").exists())
        self.assertEqual([p.name for p in (self.prefix / "lib").iterdir()], ["warp-lean"])

    def test_equal_version_is_up_to_date_without_downloading(self):
        self.install_version("0.10.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        (self.release / TAG / "warper-linux-arm64.tar.gz").unlink()
        result = self.run_linux("y\ny\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Warper 0.10.0 is up to date", result.stdout)

    def test_older_release_is_not_offered(self):
        self.install_version("1.0.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("y\ny\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("up to date", result.stdout)

    def test_declining_leaves_the_installation_untouched(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("n\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Warper 0.10.0 is available (installed: 0.9.0)", result.stdout)
        self.assertIn(f"https://github.com/ragokan/warper/releases/tag/{TAG}", result.stdout)
        self.assertIn("old", (self.app / "warp-oss").read_text())
        self.assertFalse((self.app / "warp-oss.previous").exists())

    def test_install_replaces_the_binary_and_leaves_no_previous_copy(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Checksum verified", result.stdout)
        self.assertIn("Installed Warper 0.10.0\n", result.stdout)
        self.assertNotIn("previous", result.stdout)
        self.assertIn("next time you launch", result.stdout)
        self.assertIn("new", (self.app / "warp-oss").read_text())
        self.assertEqual(
            sorted(p.name for p in self.app.iterdir()),
            [".warp-oss.old", "VERSION", "resources", "warp-oss", "warper-update"],
        )
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")
        self.assertTrue((self.app / "resources" / "schema.json").exists())
        self.assertFalse((self.app / "resources" / "old.json").exists())
        self.assertIn("new updater", (self.app / "warper-update").read_text())
        self.assertTrue(os.access(self.app / "warp-oss", os.X_OK))
        self.assertEqual([p.name for p in (self.prefix / "lib").iterdir()], ["warp-lean"])

    def test_unknown_version_asks_instead_of_calling_it_an_update(self):
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("n\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(
            "Installed version unknown (built from source); latest is 0.10.0", result.stdout
        )
        self.assertNotIn("is available", result.stdout)
        self.assertNotIn("up to date", result.stdout)
        self.assertIn("Install? [y/N]", result.stdout)
        self.assertFalse((self.app / "warp-oss.previous").exists())

    def test_unknown_version_can_be_installed_and_is_then_recorded(self):
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")

    def test_garbage_version_counts_as_unknown(self):
        self.install_version("not-a-version")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("n\n")
        self.assertIn("Installed version unknown", result.stdout)

    def test_portable_copy_without_warp_oss_is_refused(self):
        portable = self.root / "portable"
        portable.mkdir()
        shutil.copy2(SCRIPT, portable / "warper-update")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_script("y\n", script=portable / "warper-update")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("portable copies cannot be updated in place", result.stderr)

    def test_failure_after_the_binary_swap_restores_the_previous_install(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        self.shim(
            "mv",
            'echo "$@" >> "$MV_LOG"\n'
            'for last; do :; done\n'
            'case $last in */resources) echo "mv: simulated failure" >&2; exit 1 ;; esac\n'
            'command -p mv "$@"\n',
        )
        log = self.root / "mv.log"
        result = self.run_linux("y\ny\n", MV_LOG=str(log))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("the previous install was restored", result.stderr)
        swapped = log.read_text()
        self.assertIn("/new/warp-oss", swapped)
        self.assertIn("/new/VERSION", swapped)
        self.assert_old_install_intact()

    def test_failed_restore_keeps_the_previous_files_and_names_them(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        self.shim(
            "mv",
            'for last; do :; done\n'
            'case $last in */resources) echo "mv: simulated failure" >&2; exit 1 ;; esac\n'
            'case $1 in */old/warp-oss) echo "mv: simulated restore failure" >&2; exit 1 ;; esac\n'
            'command -p mv "$@"\n',
        )
        result = self.run_linux("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        match = re.search(r"could not restore automatically; your previous files are in (\S+)", result.stderr)
        self.assertIsNotNone(match, result.stderr)
        kept = Path(match[1])
        staged = [path.resolve() for path in self.staging_dirs(self.prefix / "lib")]
        self.assertEqual(staged, [kept.parent.resolve()])
        self.assertIn("old", (kept / "warp-oss").read_text())

    def test_a_previous_binary_from_an_older_updater_is_removed(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        write_executable(self.app / "warp-oss.previous", "#!/bin/sh\necho older\n")
        write_executable(self.app / "warp-oss.lean-v0.2.0", "#!/bin/sh\necho rollback\n")
        result = self.run_linux("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.app / "warp-oss.previous").exists())
        self.assertIn("rollback", (self.app / "warp-oss.lean-v0.2.0").read_text())
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")

    def test_failure_removing_a_stale_previous_binary_only_warns(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        write_executable(self.app / "warp-oss.previous", "#!/bin/sh\necho older\n")
        self.shim(
            "rm",
            'for last; do :; done\n'
            'case $last in */warp-oss.previous) echo "rm: simulated failure" >&2; exit 1 ;; esac\n'
            'command -p rm "$@"\n',
        )
        result = self.run_linux("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("could not remove", result.stderr)
        self.assertTrue((self.app / "warp-oss.previous").exists())
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")
        self.assertEqual(self.staging_dirs(self.prefix / "lib"), [])

    def test_a_symlink_to_the_script_updates_the_install_it_points_into(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        link = self.root / "bin" / "warper-update"
        link.parent.mkdir()
        link.symlink_to(self.app / "warper-update")
        result = self.run_script("y\nn\n", script=link)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")

    def test_incomplete_archive_changes_nothing(self):
        self.install_version("0.9.0")
        broken = {k: v for k, v in self.package.items() if "resources" not in k}
        self.publish("warper-linux-arm64.tar.gz", broken)
        result = self.run_linux("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("nothing was changed", result.stderr)
        self.assert_old_install_intact()

    def test_checksum_mismatch_aborts_before_installing(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package, corrupt=True)
        result = self.run_linux("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("checksum mismatch", result.stderr)
        self.assertIn("old", (self.app / "warp-oss").read_text())
        self.assertFalse((self.app / "warp-oss.previous").exists())

    def test_missing_checksum_file_aborts(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package, checksum=False)
        result = self.run_linux("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("SHA256SUMS", result.stderr)
        self.assertIn("old", (self.app / "warp-oss").read_text())

    def start_installed_warper(self):
        """Runs a real executable from the installed path, as the running Warper does."""
        binary = self.app / "warp-oss"
        binary.unlink()
        shutil.copy(shutil.which("sleep"), binary)  # copy2 would copy macOS's restricted file flag
        running = subprocess.Popen([str(binary), "300"])
        self.addCleanup(running.kill)
        time.sleep(0.5)
        if running.poll() is not None:
            self.skipTest("a copied sleep cannot run here")
        return running

    def running_exe(self, running):
        """Where the running process's executable lives now; None without /proc."""
        if not sys.platform.startswith("linux"):
            return None
        return Path(os.readlink(f"/proc/{running.pid}/exe"))

    def test_an_accepted_restart_removes_the_old_binary_before_the_new_one_starts(self):
        self.install_version("0.9.0")
        self.package["warper-linux-arm64/warper"] = (
            '#!/bin/sh\nstate=gone; [ -e "$OLD" ] && state=present\necho "$0 $state" > "$MARKER"\n'
        )
        self.publish("warper-linux-arm64.tar.gz", self.package)
        running = self.start_installed_warper()
        started, restart = self.root / "started", self.root / "restart"
        script = self.start_script(
            self.app / "warper-update", WARPER_PID=str(running.pid),
            WARPER_RESTART_FILE=str(restart), MARKER=str(started), OLD=str(self.app / ".warp-oss.old"),
        )
        self.answer(script, "y\n")
        self.read_until(script, "Restart Warper now? Open tabs will close. [y/N] ")
        self.assertEqual(self.staging_dirs(self.prefix / "lib"), [])
        self.assertFalse((self.app / "warp-oss.previous").exists())
        self.assertIsNone(running.poll(), "Warper died with its binary")
        exe = self.running_exe(running)
        if exe is not None:
            self.assertEqual(exe, self.app / ".warp-oss.old")
        self.answer(script, "y\n")
        self.assertEqual(script.wait(timeout=30), 0)
        self.assertTrue(restart.exists(), "Warper was not asked to quit")
        self.assertTrue((self.app / ".warp-oss.old").exists(), "removed while Warper still runs")
        self.assertFalse(started.exists(), "the new binary started before Warper exited")
        running.terminate()
        running.wait(timeout=20)
        self.assertTrue(self.wait_for(started), "the new binary was not launched")
        time.sleep(0.3)
        launched, state = started.read_text().split()
        self.assertEqual(Path(launched).resolve(), (self.app / "warp-oss").resolve())
        self.assertEqual(state, "gone")

    def test_a_declined_restart_keeps_the_old_binary_resolvable_until_warper_exits(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        running = self.start_installed_warper()
        script = self.start_script(self.app / "warper-update", WARPER_PID=str(running.pid))
        self.answer(script, "y\n")
        self.read_until(script, "Restart Warper now? Open tabs will close. [y/N] ")
        self.answer(script, "n\n")
        self.assertEqual(script.wait(timeout=30), 0)
        exe = self.running_exe(running)
        if exe is not None:
            self.assertEqual(exe, self.app / ".warp-oss.old")
            self.assertTrue(exe.exists())
            self.assertTrue((exe.parent / "resources" / "schema.json").exists())
            self.assertIn("new updater", (exe.parent / "warper-update").read_text())
        time.sleep(1.5)
        self.assertTrue((self.app / ".warp-oss.old").exists(), "removed while Warper still runs")
        running.terminate()
        running.wait(timeout=20)
        self.assertTrue(self.wait_until_gone(self.app / ".warp-oss.old"))
        self.assertEqual(self.staging_dirs(self.prefix / "lib"), [])
        self.assertIn("new", (self.app / "warp-oss").read_text())

    def test_closing_the_tab_at_the_restart_prompt_still_removes_the_old_binary(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        running = self.start_installed_warper()
        script = self.start_script(self.app / "warper-update", WARPER_PID=str(running.pid))
        self.answer(script, "y\n")
        self.read_until(script, "Restart Warper now? Open tabs will close. [y/N] ")
        script.send_signal(signal.SIGHUP)
        script.wait(timeout=30)
        time.sleep(1.5)
        self.assertTrue((self.app / ".warp-oss.old").exists(), "removed while Warper still runs")
        running.terminate()
        running.wait(timeout=20)
        self.assertTrue(self.wait_until_gone(self.app / ".warp-oss.old"))
        deadline = time.time() + 10
        while self.helper_pids(running) and time.time() < deadline:
            time.sleep(0.1)
        self.assertEqual(self.helper_pids(running), [])
        self.assertEqual(self.staging_dirs(self.prefix / "lib"), [])

    def test_without_a_warper_pid_the_old_binary_stays_until_the_next_update(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        result = self.run_linux("y\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("old", (self.app / ".warp-oss.old").read_text())
        newer = dict(self.package, **{"warper-linux-arm64/warper": "#!/bin/sh\necho newer\n"})
        self.publish("warper-linux-arm64.tar.gz", newer, tag="lean-v0.11.0")
        result = self.run_script("y\n", WARPER_APP=str(self.app))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("newer", (self.app / "warp-oss").read_text())
        self.assertIn("new", (self.app / ".warp-oss.old").read_text())
        self.assertNotIn("old", (self.app / ".warp-oss.old").read_text())
        self.assertEqual(self.staging_dirs(self.prefix / "lib"), [])

    def test_failure_keeping_the_old_binary_beside_the_install_only_warns(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        self.shim(
            "mv",
            'for last; do :; done\n'
            'case $last in */.warp-oss.old) echo "mv: simulated failure" >&2; exit 1 ;; esac\n'
            'command -p mv "$@"\n',
        )
        result = self.run_linux("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("could not keep the old binary", result.stderr)
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")
        self.assertIn("new", (self.app / "warp-oss").read_text())
        self.assertFalse((self.app / ".warp-oss.old").exists())
        self.assertEqual(self.staging_dirs(self.prefix / "lib"), [])

    def test_restart_waits_for_warper_to_exit_without_signalling_it(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        started, restart = self.root / "started", self.root / "restart"
        running = self.start_warper()
        result = self.run_linux(
            "y\ny\n", WARPER_PID=str(running.pid), WARPER_RESTART_FILE=str(restart),
            MARKER=str(started),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(restart.exists(), "Warper was not asked to quit")
        time.sleep(2)
        self.assertIsNone(running.poll(), "Warper was signalled")
        self.assertFalse(started.exists(), "the new binary started before Warper exited")
        running.terminate()
        running.wait(timeout=20)
        self.assertTrue(self.wait_for(started), "the new binary was not launched")

    def test_restart_gives_up_without_killing_warper_that_never_exits(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        started = self.root / "started"
        running = self.start_warper()
        result = self.run_linux(
            "y\ny\n", WARPER_PID=str(running.pid), WARPER_RESTART_WAIT="1", MARKER=str(started),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        time.sleep(4)
        self.assertIsNone(running.poll(), "Warper was signalled")
        self.assertFalse(started.exists())

    def test_helpers_get_their_own_session_without_setsid(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        tools = self.root / "tools"
        tools.mkdir()
        for directory in ("/usr/bin", "/bin"):
            for entry in os.scandir(directory):
                if entry.name != "setsid" and not (tools / entry.name).exists():
                    (tools / entry.name).symlink_to(entry.path)
        running = self.start_warper()
        result = self.run_linux(
            "y\ny\n", PATH=f"{self.shims}:{tools}", WARPER_PID=str(running.pid),
            MARKER=str(self.root / "started"),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        helpers = self.helper_pids(running)
        self.assertEqual(len(helpers), 2, "expected the old-binary cleanup and the relauncher")
        deadline = time.time() + 10
        while any(os.getsid(pid) != pid for pid in helpers) and time.time() < deadline:
            time.sleep(0.1)
        for pid in helpers:
            self.assertEqual(os.getsid(pid), pid)
        running.terminate()
        running.wait(timeout=20)

    def test_declining_the_restart_applies_the_update_next_launch(self):
        self.install_version("0.9.0")
        self.publish("warper-linux-arm64.tar.gz", self.package)
        restart = self.root / "restart"
        running = self.start_warper()
        result = self.run_linux(
            "y\nn\n", WARPER_PID=str(running.pid), WARPER_RESTART_FILE=str(restart),
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("next time you launch", result.stdout)
        self.assertFalse(restart.exists())
        self.assertIsNone(running.poll())


@unittest.skipUnless(sys.platform.startswith("linux"), "the installer is Linux-only")
class InstallScriptTests(UpdateTests):
    def setUp(self):
        super().setUp()
        self.shim("update-desktop-database", "exit 0\n")
        self.shim("xdg-mime", "exit 0\n")
        self.prefix = self.root / "prefix"
        self.app = self.prefix / "lib" / "warp-lean"
        self.package = self.root / "package"
        write_executable(
            self.package / "warper",
            '#!/bin/sh\n[ "$1" = dump-settings-schema ] && echo "{}" > "$2"\nexit 0\n',
        )

    def install(self):
        home = self.root / "home"
        return subprocess.run(
            ["bash", str(SCRIPT.parent / "install-linux.sh"), str(self.package / "warper")],
            text=True,
            capture_output=True,
            timeout=60,
            env={
                "PATH": f"{self.shims}:{os.environ['PATH']}",
                "HOME": str(home),
                "LEAN_INSTALL_PREFIX": str(self.prefix),
                "XDG_CONFIG_HOME": str(home / "config"),
                "XDG_DATA_HOME": str(home / "data"),
                "XDG_STATE_HOME": str(home / "state"),
            },
        )

    def test_a_release_package_version_is_installed(self):
        (self.package / "VERSION").write_text("0.10.0\n")
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.app / "VERSION").read_text(), "0.10.0\n")
        self.assertTrue(os.access(self.app / "warper-update", os.X_OK))

    def test_a_source_install_removes_a_stale_version(self):
        self.app.mkdir(parents=True)
        (self.app / "VERSION").write_text("0.9.0\n")
        result = self.install()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.app / "VERSION").exists())


class MacosUpdateTests(UpdateTests):
    def setUp(self):
        super().setUp()
        self.shim("uname", 'case "$1" in -s) echo Darwin ;; -m) echo arm64 ;; esac\n')
        self.shim("defaults", 'cat "${2%/Info}/Info.version" 2>/dev/null\n')
        self.shim("ditto", 'unzip -q "$3" -d "$4"\n')
        self.shim("codesign", "exit 0\n")
        self.applications = self.root / "Applications"
        self.bundle = self.applications / "Warper.app"
        (self.bundle / "Contents").mkdir(parents=True)
        (self.bundle / "Contents" / "Info.version").write_text("0.9.0\n")
        (self.bundle / "Contents" / "marker").write_text("old\n")
        self.package = {
            "Warper.app/Contents/Info.version": "0.10.0\n",
            "Warper.app/Contents/marker": "new\n",
        }

    def run_macos(self, answers, **env):
        return self.run_script(answers, WARPER_APP=str(self.bundle), **env)

    def stale_previous_bundle(self):
        stale = self.applications / "Warper.previous.app" / "Contents"
        stale.mkdir(parents=True)
        (stale / "marker").write_text("older\n")
        return stale.parent

    def test_replaces_the_bundle_and_leaves_no_previous_copy(self):
        self.publish("warper-macos-arm64.zip", self.package)
        result = self.run_macos("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("Installed Warper 0.10.0\n", result.stdout)
        self.assertNotIn("previous", result.stdout)
        self.assertEqual((self.bundle / "Contents" / "marker").read_text(), "new\n")
        self.assertEqual([path.name for path in self.applications.iterdir()], ["Warper.app"])

    def test_a_previous_bundle_from_an_older_updater_is_removed(self):
        self.stale_previous_bundle()
        other = self.applications / "Other.previous.app"
        other.mkdir()
        self.publish("warper-macos-arm64.zip", self.package)
        result = self.run_macos("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.bundle / "Contents" / "marker").read_text(), "new\n")
        self.assertEqual(
            sorted(path.name for path in self.applications.iterdir()),
            ["Other.previous.app", "Warper.app"],
        )

    def test_failure_removing_a_stale_previous_bundle_only_warns(self):
        stale = self.stale_previous_bundle()
        self.publish("warper-macos-arm64.zip", self.package)
        self.shim(
            "rm",
            'for last; do :; done\n'
            'case $last in */Warper.previous.app) echo "rm: simulated failure" >&2; exit 1 ;; esac\n'
            'command -p rm "$@"\n',
        )
        result = self.run_macos("y\nn\n")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("could not remove", result.stderr)
        self.assertEqual((stale / "Contents" / "marker").read_text(), "older\n")
        self.assertEqual((self.bundle / "Contents" / "marker").read_text(), "new\n")
        self.assertEqual(self.staging_dirs(self.applications), [])

    def test_failure_moving_the_new_bundle_in_restores_the_old_one(self):
        self.publish("warper-macos-arm64.zip", self.package)
        self.shim(
            "mv",
            'case $1 in */.warper-update.*/Warper.app) echo "mv: simulated failure" >&2; exit 1 ;; esac\n'
            'command -p mv "$@"\n',
        )
        result = self.run_macos("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("the old one was restored", result.stderr)
        self.assertEqual((self.bundle / "Contents" / "marker").read_text(), "old\n")
        self.assertEqual([path.name for path in self.applications.iterdir()], ["Warper.app"])

    def test_a_failed_restore_keeps_the_old_bundle_and_names_it(self):
        self.publish("warper-macos-arm64.zip", self.package)
        self.shim(
            "mv",
            'case $1 in */.warper-update.*/Warper.app | */.warper-update.*/old.app)\n'
            '  echo "mv: simulated failure" >&2; exit 1 ;; esac\n'
            'command -p mv "$@"\n',
        )
        result = self.run_macos("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        match = re.search(r"could not restore the old one; it is in (\S+)", result.stderr)
        self.assertIsNotNone(match, result.stderr)
        kept = Path(match[1])
        self.assertEqual((kept / "Contents" / "marker").read_text(), "old\n")
        self.assertEqual([path.resolve() for path in self.staging_dirs(self.applications)], [kept.parent.resolve()])

    def test_the_old_bundle_is_gone_at_the_restart_prompt_and_the_new_one_opens(self):
        self.publish("warper-macos-arm64.zip", self.package)
        self.shim("open", 'echo "$@" > "$OPEN_LOG"\n')
        opened, restart = self.root / "opened", self.root / "restart"
        running = self.start_warper()
        script = self.start_script(
            SCRIPT, WARPER_APP=str(self.bundle), WARPER_PID=str(running.pid),
            WARPER_RESTART_FILE=str(restart), OPEN_LOG=str(opened),
        )
        self.answer(script, "y\n")
        self.read_until(script, "Restart Warper now? Open tabs will close. [y/N] ")
        self.assertEqual(self.staging_dirs(self.applications), [])
        self.assertEqual([path.name for path in self.applications.iterdir()], ["Warper.app"])
        self.answer(script, "y\n")
        self.assertEqual(script.wait(timeout=30), 0)
        self.assertTrue(restart.exists(), "Warper was not asked to quit")
        running.terminate()
        running.wait(timeout=20)
        self.assertTrue(self.wait_for(opened), "the new bundle was not opened")
        self.assertTrue(self.wait_for(opened))
        time.sleep(0.5)
        self.assertEqual(opened.read_text().strip(), str(self.bundle))

    def test_checksum_mismatch_keeps_the_installed_bundle(self):
        self.publish("warper-macos-arm64.zip", self.package, corrupt=True)
        result = self.run_macos("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual((self.bundle / "Contents" / "marker").read_text(), "old\n")

    @unittest.skipIf(os.geteuid() == 0, "root can write anywhere")
    def test_unwritable_location_explains_and_stops(self):
        self.publish("warper-macos-arm64.zip", self.package)
        self.applications.chmod(stat.S_IRUSR | stat.S_IXUSR)
        self.addCleanup(self.applications.chmod, stat.S_IRWXU)
        result = self.run_macos("y\ny\n")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("is not writable", result.stderr)
        self.assertEqual((self.bundle / "Contents" / "marker").read_text(), "old\n")


if __name__ == "__main__":
    unittest.main()
