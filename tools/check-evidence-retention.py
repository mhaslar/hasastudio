#!/usr/bin/env python3
"""Reject deleted historical evidence paths; compare Git trees, not worktrees."""
import argparse
import json
import subprocess


def deleted_evidence(base, head, cwd=None):
    """Renames count as deletions because historical links still need old paths."""
    for revision in (base, head):
        subprocess.run(['git', 'rev-parse', '--verify', '--end-of-options', revision + '^{commit}'],
                       cwd=cwd, check=True, stdout=subprocess.DEVNULL)
    data = subprocess.check_output(
        ['git', 'diff', '--no-renames', '--diff-filter=D', '--name-only', '-z',
         base, head, '--', 'docs/testing/', 'docs/benchmarks/'], cwd=cwd)
    return [p.decode('utf-8', errors='surrogateescape') for p in data.split(b'\0') if p]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True)
    parser.add_argument('--head', default='HEAD')
    args = parser.parse_args()
    removed = deleted_evidence(args.base, args.head)
    if removed:
        raise SystemExit('Historical evidence paths were removed: ' + json.dumps(removed)
                         + '\nRestore the original files; store new runs at new paths. See ADR 0036.')
    print('PASS: no historical evidence paths removed.')


if __name__ == '__main__':
    main()
