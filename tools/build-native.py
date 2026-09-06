#!/usr/bin/env python3
"""Build the approved decode dependencies; never select system FFmpeg."""
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DEPS = ROOT / '.deps'
PREFIX = DEPS / 'native'


def run(args, **kw):
    print('+', ' '.join(map(str, args)), flush=True)
    subprocess.run(list(map(str, args)), check=True, **kw)


def verified(name):
    manifest = json.loads((ROOT / 'xtask/dependencies.json').read_text())
    item = next(d for d in manifest['dependencies'] if d['name'] == name)
    path = DEPS / item['filename']
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    if digest != item['sha256']:
        raise RuntimeError(f'{path}: expected {item["sha256"]}, got {digest}')
    return path


def environment():
    env = os.environ.copy()
    env.pop('FFMPEG_DIR', None)
    if os.name == 'nt':
        env['FFMPEG_DIR'] = str(PREFIX)
    else:
        env['PKG_CONFIG_PATH'] = str(PREFIX / 'lib/pkgconfig')
        env['LD_LIBRARY_PATH'] = str(PREFIX / 'lib')
        env['DYLD_LIBRARY_PATH'] = str(PREFIX / 'lib')
    env['PATH'] = str(PREFIX / 'bin') + os.pathsep + env['PATH']
    return env


def main():
    identity = hashlib.sha256((ROOT / 'xtask/dependencies.json').read_bytes()
                              + Path(__file__).read_bytes()
                              + (ROOT / 'tools/patches/ffmpeg-7.1.1-vt-hardware.patch').read_bytes()
                              + platform.platform().encode()).hexdigest()
    marker = PREFIX / 'rezie-build.json'
    if marker.exists() and json.loads(marker.read_text())['identity'] == identity:
        print('Native prefix matches the pinned recipe; build/startup probes still run.')
    else:
        # Only this disposable, generated prefix is rebuilt. Evidence is elsewhere.
        if PREFIX.exists():
            shutil.rmtree(PREFIX)
        PREFIX.mkdir(parents=True)
        if os.name == 'nt':
            with zipfile.ZipFile(verified('ffmpeg')) as archive:
                for item in archive.infolist():
                    parts = Path(item.filename).parts[1:]
                    if not parts or item.is_dir():
                        continue
                    destination = PREFIX.joinpath(*parts)
                    if not destination.resolve().is_relative_to(PREFIX.resolve()):
                        raise RuntimeError('unsafe archive member')
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_bytes(archive.read(item))
        else:
            tools = DEPS / 'build-tools'
            if not (tools / 'bin/python').exists():
                run([sys.executable, '-m', 'venv', tools])
            run([tools / 'bin/python', '-m', 'pip', 'install',
                 '--disable-pip-version-check', 'meson==1.9.1', 'ninja==1.13.0'])
            source = DEPS / 'native-source'
            source.mkdir(exist_ok=True)
            for name in ('dav1d', 'ffmpeg-source'):
                with tarfile.open(verified(name)) as archive:
                    archive.extractall(source, filter='data')
            env = environment()
            env['PATH'] = str(tools / 'bin') + os.pathsep + env['PATH']
            jobs = str(min(os.cpu_count() or 2, 8))
            dav_build = source / 'dav1d-build'
            if dav_build.exists():
                shutil.rmtree(dav_build)
            run(['meson', 'setup', dav_build, source / 'dav1d-1.5.4',
                 '--prefix=' + str(PREFIX), '--libdir=lib', '--buildtype=release',
                 '--default-library=shared', '-Denable_tools=false',
                 '-Denable_tests=false'], env=env)
            run(['ninja', '-C', dav_build, '-j', jobs, 'install'], env=env)
            ffmpeg = source / 'ffmpeg-7.1.1'
            if sys.platform == 'darwin':
                run(['patch','--fuzz=0','-p1','-i',ROOT/'tools/patches/ffmpeg-7.1.1-vt-hardware.patch'],cwd=ffmpeg)
            platform_flags = (['--extra-version=rezie-vt-probe1', '--enable-videotoolbox',
                               '--enable-hwaccel=h264_videotoolbox,hevc_videotoolbox']
                              if sys.platform == 'darwin' else
                              ['--enable-vaapi', '--enable-hwaccel=h264_vaapi,hevc_vaapi,vp9_vaapi,av1_vaapi'])
            run([ffmpeg / 'configure', '--prefix=' + str(PREFIX),
                 '--enable-shared', '--disable-static', '--disable-gpl',
                 '--disable-nonfree', '--disable-version3', '--disable-autodetect',
                 '--disable-everything', '--disable-doc', '--disable-debug',
                 '--disable-avdevice', '--disable-avfilter', '--disable-swscale',
                 '--disable-swresample', '--disable-postproc', '--disable-network',
                 '--disable-programs', '--enable-ffprobe', '--enable-pthreads',
                 '--enable-libdav1d', '--enable-decoder=h264,hevc,vp9,av1,libdav1d',
                 '--enable-parser=h264,hevc,vp9,av1',
                 '--enable-demuxer=mov,matroska,mpegts', '--enable-protocol=file',
                 '--extra-ldflags=-Wl,-rpath,' + str(PREFIX / 'lib'),
                 *platform_flags], cwd=ffmpeg, env=env)
            run(['make', '-j', jobs], cwd=ffmpeg, env=env)
            run(['make', 'install'], cwd=ffmpeg, env=env)
            notices = PREFIX / 'notices'
            notices.mkdir(exist_ok=True)
            shutil.copyfile(source / 'dav1d-1.5.4/COPYING', notices / 'dav1d-COPYING')
            shutil.copyfile(ffmpeg / 'COPYING.LGPLv2.1', notices / 'FFmpeg-COPYING.LGPLv2.1')
            if sys.platform == 'darwin':
                shutil.copyfile(ROOT/'tools/patches/ffmpeg-7.1.1-vt-hardware.patch',notices/'ffmpeg-7.1.1-vt-hardware.patch')
        marker.write_text(json.dumps({'identity': identity, 'platform': platform.platform()}, indent=2) + '\n')
    env = environment()
    keys = ['FFMPEG_DIR'] if os.name == 'nt' else ['PKG_CONFIG_PATH', 'LD_LIBRARY_PATH', 'DYLD_LIBRARY_PATH']
    if os.getenv('GITHUB_ENV'):
        with open(os.environ['GITHUB_ENV'], 'a') as output:
            for key in keys:
                output.write(key + '=' + env[key] + '\n')
        with open(os.environ['GITHUB_PATH'], 'a') as output:
            output.write(str(PREFIX / 'bin') + '\n')
    if os.name == 'nt':
        (DEPS / 'native-env.ps1').write_text(
            "$env:FFMPEG_DIR = Join-Path $PSScriptRoot 'native'\n"
            "$env:PATH = (Join-Path $env:FFMPEG_DIR 'bin') + ';' + $env:PATH\n")
        print('Next: . .\\.deps\\native-env.ps1')
    else:
        import shlex
        (DEPS / 'native-env.sh').write_text(''.join(
            'export ' + key + '=' + shlex.quote(env[key]) + '\n' for key in keys))
        print('Next: source .deps/native-env.sh')


if __name__ == '__main__':
    main()
