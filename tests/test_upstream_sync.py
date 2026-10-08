# SPDX-FileCopyrightText: 2026 Popbones
# SPDX-License-Identifier: EUPL-1.2
"""Exercise the workflow's real rebase script against disposable local repositories."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / '.github/scripts/rebase-upstream.sh'
PUBLISH = SCRIPT.with_name('publish-rebase.sh')


class UpstreamSyncTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='eza-upstream-sync-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.upstream = self.root / 'upstream'
        self.fork = self.root / 'fork'
        self.env = {
            **os.environ,
            'GIT_AUTHOR_NAME': 'Test', 'GIT_AUTHOR_EMAIL': 'test@example.invalid',
            'GIT_COMMITTER_NAME': 'Test', 'GIT_COMMITTER_EMAIL': 'test@example.invalid',
            'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': os.devnull,
            'UPSTREAM_URL': str(self.upstream),
            'UPSTREAM_BRANCH': 'main',
            'GITHUB_OUTPUT': str(self.root / 'outputs'),
            'GITHUB_STEP_SUMMARY': str(self.root / 'summary'),
        }
        self.git(self.root, 'init', '-b', 'main', str(self.upstream))
        self.commit(self.upstream, 'shared', 'base\n')
        self.git(self.root, 'clone', str(self.upstream), str(self.fork))
        self.git(self.fork, 'switch', '-c', 'popbones/develop')

    def git(self, cwd, *args):
        result = subprocess.run(['git', *args], cwd=cwd, env=self.env,
                                check=True, capture_output=True, text=True)
        return result.stdout.strip()

    def commit(self, repo, name, contents):
        (repo / name).write_text(contents)
        self.git(repo, 'add', name)
        self.git(repo, 'commit', '-m', name)
        return self.git(repo, 'rev-parse', 'HEAD')

    def rebase(self):
        return subprocess.run(['bash', str(SCRIPT)], cwd=self.fork, env=self.env,
                              capture_output=True, text=True)

    def test_replays_personal_commit_and_preserves_upstream(self):
        self.commit(self.fork, 'personal', 'personal feature\n')
        before = self.git(self.fork, 'rev-parse', 'HEAD')
        upstream = self.commit(self.upstream, 'upstream-change', 'upstream\n')
        result = self.rebase()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.git(self.fork, 'merge-base', '--is-ancestor', upstream, 'HEAD')
        self.assertEqual((self.fork / 'personal').read_text(), 'personal feature\n')
        self.assertEqual(self.git(self.upstream, 'rev-parse', 'HEAD'), upstream)
        outputs = (self.root / 'outputs').read_text()
        self.assertIn(f'before_sha={before}', outputs)
        self.assertIn(f'after_sha={self.git(self.fork, "rev-parse", "HEAD")}', outputs)

    def test_conflict_aborts_and_restores_original_head(self):
        before = self.commit(self.fork, 'shared', 'personal\n')
        upstream = self.commit(self.upstream, 'shared', 'conflicting upstream\n')
        result = self.rebase()
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.git(self.fork, 'rev-parse', 'HEAD'), before)
        self.assertEqual(self.git(self.fork, 'status', '--porcelain'), '')
        self.assertEqual(self.git(self.upstream, 'rev-parse', 'HEAD'), upstream)
        self.assertIn('shared', (self.root / 'summary').read_text())
        self.assertNotIn('after_sha=', (self.root / 'outputs').read_text())

    def test_noop_keeps_head(self):
        before = self.git(self.fork, 'rev-parse', 'HEAD')
        result = self.rebase()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git(self.fork, 'rev-parse', 'HEAD'), before)

    def test_dirty_checkout_is_rejected_without_changing_head(self):
        before = self.git(self.fork, 'rev-parse', 'HEAD')
        (self.fork / 'untracked').write_text('keep me\n')
        self.assertNotEqual(self.rebase().returncode, 0)
        self.assertEqual(self.git(self.fork, 'rev-parse', 'HEAD'), before)
        self.assertEqual((self.fork / 'untracked').read_text(), 'keep me\n')

    def publish_candidate(self, concurrent=False):
        before = self.commit(self.fork, 'personal', 'personal feature\n')
        remote = self.root / 'remote.git'
        self.git(self.root, 'clone', '--bare', str(self.fork), str(remote))
        self.git(self.fork, 'remote', 'set-url', 'origin', str(remote))
        self.commit(self.upstream, 'upstream-change', 'upstream\n')
        self.assertEqual(self.rebase().returncode, 0)
        candidate = self.git(self.fork, 'rev-parse', 'HEAD')
        concurrent_sha = None
        if concurrent:
            other = self.root / 'other'
            self.git(self.root, 'clone', str(remote), str(other))
            concurrent_sha = self.commit(other, 'concurrent', 'keep this work\n')
            self.git(other, 'push', 'origin', 'popbones/develop')
        result = subprocess.run(['bash', str(PUBLISH)], cwd=self.fork,
                                env={**self.env, 'BEFORE_SHA': before,
                                     'AFTER_SHA': candidate},
                                capture_output=True, text=True)
        return remote, before, candidate, concurrent_sha, result

    def test_publish_atomically_retains_original_head_for_recovery(self):
        remote, before, candidate, _, result = self.publish_candidate()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.git(remote, 'rev-parse', 'popbones/develop'), candidate)
        self.assertEqual(self.git(remote, 'rev-parse', 'popbones/previous'), before)

    def test_publish_refuses_concurrent_change_without_creating_recovery_ref(self):
        remote, _, _, concurrent, result = self.publish_candidate(concurrent=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.git(remote, 'rev-parse', 'popbones/develop'), concurrent)
        self.assertEqual(self.git(remote, 'for-each-ref', '--format=%(refname)',
                                  'refs/heads/popbones/previous'), '')


if __name__ == '__main__':
    unittest.main()
