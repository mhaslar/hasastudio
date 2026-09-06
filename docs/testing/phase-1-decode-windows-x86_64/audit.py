"""Recheck retained Windows decode evidence against the independent fixture oracle.

Run from any directory with Python 3; no native decoder or GPU is invoked.
The report's producer-reported pass flags are deliberately not used as proof.
"""
import hashlib
import json
from pathlib import Path
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(condition, message):
    if not condition:
        raise ValueError(message)


def main():
    fixture_dir = ROOT / 'tests/assets/phase-1/decode'
    oracle_path = fixture_dir / 'manifest.json'
    oracle = json.loads(oracle_path.read_text())['files']
    artifact = json.loads((HERE.parent / 'phase-1-ffmpeg-windows-artifact-audit.json').read_text())
    mac = json.loads((HERE.parent / 'phase-1-decode-macos-aarch64/auto/report.json').read_text())
    results = []
    for mode in ('hardware', 'software'):
        directory = HERE / mode
        report = json.loads((directory / 'report.json').read_text())
        source = report['source']['git_revision']
        check(source == 'e9c76054360cb24594fff47a9a92483c055354e3', 'unexpected source revision')
        check(report['git_revision'] == source, 'source revisions disagree')
        check(report['os'] == 'windows' and report['architecture'] == 'x86_64', 'wrong target')
        check(report['mode'] == ('RequireHardware' if mode == 'hardware' else 'Auto'), 'wrong mode')
        host = report['reference_host']
        check('Windows 11' in host['os']['Caption'], 'wrong reference OS')
        check(any('RX 6800 XT' in g['Name'] for g in host['gpu']), 'missing reference GPU')
        runtime, build = report['runtime_native'], report['build_native']
        check(runtime['version'] == int(build['version']) == 4002661, 'native versions differ')
        check(runtime['version'] >> 16 == 61, 'wrong native major')
        check(runtime['licence'] == build['licence'] == 'LGPL version 3 or later', 'wrong licence')
        check(runtime['configuration'] == build['configuration'] == artifact['embedded_configuration'], 'native configuration differs from inspected pin')
        check('--enable-gpl' not in runtime['configuration'] and '--enable-nonfree' not in runtime['configuration'], 'forbidden configuration')
        check(len(report['cases']) == len(oracle) == 7, 'incomplete fixture inventory')
        cases = []
        for case, expected, mac_case in zip(report['cases'], oracle, mac['cases']):
            name = expected['file']
            check(case['input'] == mac_case['input'] == name, 'fixture order differs')
            check(case['input_sha256'] == expected['sha256'] == sha(fixture_dir / name), name + ': fixture hash')
            committed = subprocess.check_output(['git', 'show', source + ':tests/assets/phase-1/decode/' + name], cwd=ROOT)
            check(hashlib.sha256(committed).hexdigest() == expected['sha256'], name + ': source fixture hash')
            pictures = case['pictures']
            check(len(pictures) == len(expected['pictures']) == len(mac_case['pictures']) == 24, name + ': count')
            for index, (actual, want, mac_picture) in enumerate(zip(pictures, expected['pictures'], mac_case['pictures'])):
                check(actual['index'] == index, name + ': missing or reordered index')
                for field in ('pts', 'time_base', 'component_sha256', 'component_depth', 'dimensions'):
                    check(actual[field] == want[field] == mac_picture[field], f'{name} picture {index}: {field}')
                for field in ('duration', 'has_alpha', 'interlaced', 'colour_primaries', 'colour_range', 'colour_space', 'colour_transfer', 'chroma_location'):
                    check(actual[field] == mac_picture[field], f'{name} picture {index}: cross-platform {field}')
            status = case['status']
            if mode == 'hardware':
                check(status['hardware_frame_context_observed'] is True, name + ': no hardware frame context')
                check(status['hardware_device'] == status['observed_hardware_device'] == 'd3d11va', name + ': wrong hardware device')
                check(status['hardware_pixel_format'] == 'd3d11', name + ': wrong hardware pixel format')
                check(status['fallback_reason'] is None and status['environment_disabled_hardware'] is False, name + ': hardware fallback')
                check(status['decoder'] == name.split('.')[0].replace('-10', '').replace('10', ''), name + ': hardware decoder')
            else:
                check(status['environment_disabled_hardware'] is True, name + ': override absent')
                check(status['hardware_frame_context_observed'] is False, name + ': unexpected hardware frame')
                check(all(status[k] is None for k in ('hardware_device', 'observed_hardware_device', 'hardware_pixel_format')), name + ': unexpected hardware')
                decoder = 'libdav1d' if name.startswith('av1') else name.split('.')[0].replace('10', '')
                check(status['decoder'] == decoder, name + ': software decoder')
            cases.append(dict(input=name, pictures=len(pictures), exact_oracle_and_mac_match=True,
                              readback_formats=sorted({p['pixel_format'] for p in pictures}), status=status))
        native_log = (directory / 'native.log').read_text()
        check('libdav1d 7161642' in native_log, 'native dav1d identifier missing')
        results.append(dict(mode=mode, source=source, source_worktree=report['source']['git_worktree'],
                            host=host, pictures=168, cases=cases,
                            evidence_sha256={p.name: sha(p) for p in sorted(directory.iterdir()) if p.is_file()},
                            libavcodec='61.19.101', licence=runtime['licence'], dav1d_reported_identifier='7161642'))
    out = dict(scope='Offline audit of retained Windows native decode evidence; no performance, preview or Phase 1 closure claim',
               passed=True, fixture_oracle_sha256=sha(oracle_path), runs=results,
               limitations=['Component hashes are observed decoded samples, not serialized raw YUV planes.',
                            'Native libclang version is not recorded; rustc LLVM 22.1.8 is a separate compiler backend.',
                            '7161642 is retained as a native dav1d build identifier, not interpreted as a semantic release version.',
                            'Host inventory identifies RX 6800 XT; per-decoder DXGI adapter identity is not serialized.',
                            'Both source worktrees record deletion of two unrelated golden audit JSON files, restored byte-for-byte in the audit commit.'])
    (HERE / 'audit.json').write_text(json.dumps(out, indent=2) + '\n')
    print('PASS: 336 Windows pictures; exact oracle and Mac PTS/component hashes; hardware and software status verified.')


if __name__ == '__main__':
    main()
