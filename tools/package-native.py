#!/usr/bin/env python3
"""Place approved shared libraries beside development binaries and relocate them."""
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]


def run(args):
    subprocess.run(list(map(str, args)), check=True, capture_output=True)


def main():
    output, app, headless = map(Path, sys.argv[1:])
    prefix = ROOT/'.deps/native'
    libraries = output if os.name == 'nt' else output/'lib'
    libraries.mkdir(parents=True, exist_ok=True)
    source = prefix/('bin' if os.name == 'nt' else 'lib')
    copied = []
    for path in source.iterdir():
        if (os.name == 'nt' and path.suffix.lower() == '.dll') or (os.name != 'nt' and ('.dylib' in path.name or '.so' in path.name)):
            destination = libraries/path.name
            if destination.exists() or destination.is_symlink():
                destination.unlink()
            if path.is_symlink():
                destination.symlink_to(os.readlink(path))
            else:
                shutil.copyfile(path, destination)
                copied.append(destination)
    notices = output/'native-notices'
    notices.mkdir(exist_ok=True)
    if (prefix/'notices').exists():
        shutil.copytree(prefix/'notices', notices, dirs_exist_ok=True)
    if (prefix/'LICENSE.txt').exists():
        shutil.copyfile(prefix/'LICENSE.txt', notices/'FFmpeg-LGPLv3.txt')
    shutil.copyfile(ROOT/'xtask/dependencies.json', notices/'dependency-manifest.json')
    if sys.platform == 'darwin':
        for binary in copied + [app, headless]:
            text = subprocess.check_output(['otool','-L',binary],text=True)
            for line in text.splitlines()[1:]:
                dependency = line.strip().split(' (')[0]
                if dependency.startswith(str(source)+'/'):
                    relative = os.path.relpath(libraries/Path(dependency).name, binary.parent)
                    run(['install_name_tool','-change',dependency,'@loader_path/'+relative,binary])
            if binary in copied:
                run(['install_name_tool','-id','@loader_path/'+binary.name,binary])
            run(['codesign','--force','--sign','-',binary])
    elif os.name != 'nt':
        for binary in copied:
            run(['patchelf','--set-rpath','$ORIGIN',binary])
        for binary in [app, headless]:
            run(['patchelf','--set-rpath','$ORIGIN/lib',binary])
    (notices/'REVIEW.txt').write_text(
        'Development validation package only. Before distribution, complete SPEC section 16 item 4: '
        'exact corresponding sources, all bundled component notices and commercial licence review. '
        'The Windows aggregate LGPLv3 notice alone is not a completed third-party notice audit.\n')


if __name__ == '__main__':
    main()
