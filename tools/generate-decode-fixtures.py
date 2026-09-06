#!/usr/bin/env python3
"""One-time test-media authoring, never part of the application build/bootstrap.

The external encoder executable may contain GPL encoders. No encoder code or
library is copied/linked into Rezie; output bitstreams contain our own pixels.
Keep its exact version/configuration in the manifest. Normal tests use committed
files and the approved LGPL decoder, never this authoring executable.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(args, **kw):
    return subprocess.run(list(map(str, args)), check=True, capture_output=True, **kw).stdout


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--ffmpeg', required=True)
    args = parser.parse_args()
    ffmpeg = Path(args.ffmpeg)
    probe = ffmpeg.with_name('ffprobe' + ffmpeg.suffix)
    output = ROOT / 'tests/assets/phase-1/decode'
    if output.exists():
        raise RuntimeError('fixture directory exists; do not overwrite evidence')
    output.mkdir()
    width, height, count = 160, 96, 24
    data = bytearray()
    for n in range(count):
        for plane, w, h in [(0, width, height), (1, width//2, height//2), (2, width//2, height//2)]:
            data.extend(16 + ((x*3 + y*5 + n*11 + plane*47) % 220)
                        for y in range(h) for x in range(w))
    author = run([ffmpeg, '-version']).decode()
    files = []
    with tempfile.TemporaryDirectory() as temporary:
        raw = Path(temporary) / 'input.yuv'
        raw.write_bytes(data)
        raw10 = Path(temporary) / 'input10.yuv'
        raw10.write_bytes(b''.join((v*4 + i%4).to_bytes(2,'little') for i,v in enumerate(data)))
        for file, codec, options in [
            ('h264.mp4', 'libx264', ['-crf','18','-bf','3']),
            ('hevc.mov', 'libx265', ['-crf','18','-x265-params','pools=none:frame-threads=1:log-level=error']),
            ('vp9.mkv', 'libvpx-vp9', ['-crf','24','-b:v','0','-deadline','good','-cpu-used','4']),
            ('av1.mkv', 'libsvtav1', ['-crf','24','-preset','12','-svtav1-params','lp=1']),
            ('hevc10.mov', 'libx265', ['-crf','18','-x265-params','pools=none:frame-threads=1:log-level=error']),
            ('av1-10.mkv', 'libsvtav1', ['-crf','24','-preset','12','-svtav1-params','lp=1']),
        ]:
            ten = '10' in file
            command = [ffmpeg, '-v','error','-f','rawvideo','-pixel_format', 'yuv420p10le' if ten else 'yuv420p',
                       '-video_size',f'{width}x{height}','-framerate','50','-i',raw10 if ten else raw,
                       '-an','-c:v',codec,'-threads','1',*options,
                       '-colorspace','bt709','-color_primaries','bt709','-color_trc','bt709',output/file]
            run(command)
            files.append(file)
        run([ffmpeg,'-v','error','-i',output/'h264.mp4','-c:v','copy','-an',output/'h264.ts'])
        files.append('h264.ts')
        records = []
        for file in files:
            path = output/file
            metadata = json.loads(run([probe,'-v','error','-select_streams','v:0','-show_streams','-show_frames','-of','json',path]))
            stream = metadata['streams'][0]
            tb = list(map(int, stream['time_base'].split('/')))
            depth = 10 if '10' in file else 8
            sample_bytes = 2 if depth == 10 else 1
            pixels = run([ffmpeg,'-v','error','-i',path,'-map','0:v:0','-fps_mode','passthrough','-pix_fmt','yuv420p10le' if depth == 10 else 'yuv420p','-f','rawvideo','pipe:1'])
            size = width*height*3//2*sample_bytes
            assert len(pixels) == count*size and len(metadata['frames']) == count
            pictures = []
            for i, frame in enumerate(metadata['frames']):
                offset = i*size
                hashes = []
                for plane_size in [width*height, width*height//4, width*height//4]:
                    plane_size *= sample_bytes
                    plane = pixels[offset:offset+plane_size]
                    if depth == 8:
                        canonical = bytearray(2*len(plane))
                        canonical[0::2] = plane
                    else:
                        canonical = plane
                    hashes.append(hashlib.sha256(canonical).hexdigest())
                    offset += plane_size
                pictures.append(dict(pts=frame['pts'], time_base=tb, component_sha256=hashes,
                                     component_depth=[depth,depth,depth], dimensions=[width,height]))
            records.append(dict(file=file, sha256=hashlib.sha256(path.read_bytes()).hexdigest(), pictures=pictures))
    manifest = dict(scope='Owned synthetic test pixels, encoded once by a standalone authoring tool; no encoder library is an application dependency',
                    source='Deterministic YUV pattern in tools/generate-decode-fixtures.py',
                    authoring_ffmpeg=author, oracle='Independent external ffprobe PTS and ffmpeg raw YUV decode, no Rezie code', files=records)
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')


if __name__ == '__main__':
    main()
