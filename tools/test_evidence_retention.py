"""Real Git-tree tests for retention across additions, deletions and renames."""
import importlib.util
from pathlib import Path
import subprocess
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('retention', Path(__file__).with_name('check-evidence-retention.py'))
retention = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(retention)


class RetentionTests(unittest.TestCase):
    def test_tree_changes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root, stderr=subprocess.DEVNULL).decode().strip()

            def commit():
                git('add', '-A')
                git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid',
                    '-c', 'commit.gpgsign=false', 'commit', '-qm', 'fixture')
                return git('rev-parse', 'HEAD')

            git('init', '-q')
            report = root / 'docs/testing/run with spaces/report.json'
            report.parent.mkdir(parents=True)
            report.write_text('{}')
            other = root / 'scratch.txt'
            other.write_text('temporary')
            base = commit()
            (report.parent / 'new.json').write_text('{}')
            other.unlink()
            added = commit()
            self.assertEqual(retention.deleted_evidence(base, added, root), [])
            report.rename(report.with_name('renamed.json'))
            renamed = commit()
            self.assertEqual(retention.deleted_evidence(base, renamed, root),
                             ['docs/testing/run with spaces/report.json'])
            report.with_name('renamed.json').unlink()
            deleted = commit()
            self.assertEqual(retention.deleted_evidence(renamed, deleted, root),
                             ['docs/testing/run with spaces/renamed.json'])


if __name__ == '__main__':
    unittest.main()
