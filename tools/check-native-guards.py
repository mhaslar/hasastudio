#!/usr/bin/env python3
"""Exercise actual build/startup guard code against incompatible shared libraries.

No bypass exists in production code. Fixtures are isolated from .deps/native.
The startup helper compiles the same runtime_policy.rs and calls its linked
symbols. The build helper executes the unchanged rezie-media/build.rs.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(args, env, **kw):
    return subprocess.run(list(map(str, args)), env=env, capture_output=True, text=True, **kw)


def require(result):
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)


def main():
    records = []
    with tempfile.TemporaryDirectory(prefix='rezie-native-guards-') as directory:
        work = Path(directory)
        package = work/'build-probe'
        (package/'src').mkdir(parents=True)
        for name in ('build.rs', 'src/policy.rs'):
            shutil.copyfile(ROOT/'crates/rezie-media'/name, package/name)
        (package/'src/lib.rs').write_text('// Build-script integration probe.\n')
        (package/'Cargo.toml').write_text('''[package]
name = "rezie-native-build-probe"
version = "0.0.0"
edition = "2021"
[workspace]
[build-dependencies]
libloading = "=0.8.9"
pkg-config = "=0.3.32"
''')
        policy = json.dumps(str(ROOT/'crates/rezie-media/src/policy.rs'))
        runtime = json.dumps(str(ROOT/'crates/rezie-media/src/runtime_policy.rs'))
        helper = work/'startup.rs'
        helper.write_text(f'''#[path={policy}] mod policy;
#[path={runtime}] mod runtime_policy;
fn main() {{ if let Err(e) = runtime_policy::check() {{ eprintln!("{{e}}"); std::process::exit(1); }} }}
''')
        for name, major, flags, licence, expected in [
            ('lgpl21',61,'--enable-shared','LGPL version 2.1 or later',True),
            ('lgpl3',61,'--enable-shared --enable-version3','LGPL version 3 or later',True),
            ('gpl',61,'--enable-shared --enable-gpl','GPL version 3 or later',False),
            ('nonfree',61,'--enable-shared --enable-nonfree','nonfree and unredistributable',False),
            ('wrong-major',63,'--enable-shared','LGPL version 2.1 or later',False),
        ]:
            prefix = work/name
            (prefix/'lib/pkgconfig').mkdir(parents=True)
            (prefix/'bin').mkdir()
            source = prefix/'fixture.c'
            source.write_text(f'''#ifdef _WIN32
#define API __declspec(dllexport)
#else
#define API
#endif
API unsigned avcodec_version(void) {{ return {major}u << 16; }}
API const char *avcodec_configuration(void) {{ return "{flags}"; }}
API const char *avcodec_license(void) {{ return "{licence}"; }}
''')
            env = os.environ.copy()
            for key in ('FFMPEG_DIR','VCPKG_ROOT','PKG_CONFIG_LIBDIR'):
                env.pop(key,None)
            env['PKG_CONFIG_PATH'] = str(prefix/'lib/pkgconfig')
            (prefix/'lib/pkgconfig/libavcodec.pc').write_text(
                f'includedir={ROOT}/.deps/native/include\nlibdir={prefix}/lib\nName: libavcodec\nDescription: policy fixture\nVersion: {major}.0.0\nLibs: -L${{libdir}} -lavcodec\n')
            if os.name == 'nt':
                env['FFMPEG_DIR'] = str(prefix)
                env['PATH'] = str(prefix/'bin') + os.pathsep + env['PATH']
                require(run(['clang','-shared',source,'-o',prefix/'bin/avcodec-61.dll',
                             '-Wl,/IMPLIB:' + str(prefix/'lib/avcodec.lib')],env))
            elif os.uname().sysname == 'Darwin':
                env['DYLD_LIBRARY_PATH'] = str(prefix/'lib')
                require(run(['cc','-dynamiclib',source,'-o',prefix/'lib/libavcodec.dylib'],env))
            else:
                env['LD_LIBRARY_PATH'] = str(prefix/'lib')
                require(run(['cc','-shared','-fPIC',source,'-o',prefix/'lib/libavcodec.so'],env))
            executable = prefix/('startup.exe' if os.name == 'nt' else 'startup')
            require(run(['rustc','--edition=2021',helper,'-L',str(prefix/'lib'),'-l','dylib=avcodec','-o',executable],env))
            startup = run([executable],env)
            build = run(['cargo','check','--offline','--manifest-path',package/'Cargo.toml'],env)
            for stage, result in [('startup',startup),('build',build)]:
                if (result.returncode == 0) != expected:
                    raise RuntimeError(f'{stage} {name}: unexpected result\n{result.stdout}\n{result.stderr}')
                if not expected and 'libavcodec rejected:' not in result.stderr:
                    raise RuntimeError(f'{stage} {name}: failed for the wrong reason\n{result.stderr}')
                if not expected and ('expected major 61' not in result.stderr or flags not in result.stderr):
                    raise RuntimeError(f'{stage} {name}: missing actionable native properties')
                records.append(dict(case=name,stage=stage,expected_acceptance=expected,
                                    exit_code=result.returncode,passed=True))
    output = ROOT/'target/native-guard-tests.json'
    output.parent.mkdir(exist_ok=True)
    output.write_text(json.dumps(dict(os=os.name,tests=records,passed=True),indent=2)+'\n')
    print(f'All {len(records)} native guard cases passed; {output}')


if __name__ == '__main__':
    main()
