#!/usr/bin/env python3
"""Offline audit of the five Phase 1 colour scenes; Python 3.9+ stdlib only.

Never imports the Rust checker or trusts its PASS field. PNG parsing is restricted
 to this fixture's non-interlaced RGBA8/16, per https://www.w3.org/TR/png-3/#9Filters.
Raw working samples are decoded independently with struct's IEEE binary16 codec.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import subprocess
import zlib

WIDTH, HEIGHT = 257, 65
BACKGROUNDS = {
    'black': [0, 0, 0, 255], 'white': [255, 255, 255, 255],
    'colour': [31, 153, 219, 255], 'translucent': [31, 153, 219, 96],
    'transparent': [255, 0, 255, 0],
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def predictor(kind, left, above, corner):
    if kind == 0:
        return 0
    if kind == 1:
        return left
    if kind == 2:
        return above
    if kind == 3:
        return (left + above) // 2
    require(kind == 4, 'invalid PNG filter')
    p = left + above - corner
    # PNG Paeth tie order: left, above, upper-left.
    return min((left, above, corner), key=lambda v: abs(p - v))


def decode_png(data, depth):
    require(data[:8] == b'\x89PNG\r\n\x1a\n', 'invalid PNG signature')
    offset, compressed, tags = 8, bytearray(), []
    while offset < len(data):
        require(offset + 12 <= len(data), 'truncated PNG chunk')
        size = struct.unpack_from('>I', data, offset)[0]
        kind = data[offset + 4:offset + 8]
        end = offset + 8 + size
        require(end + 4 <= len(data), 'truncated PNG payload')
        payload = data[offset + 8:end]
        require(zlib.crc32(kind + payload) == struct.unpack_from('>I', data, end)[0], 'PNG CRC mismatch')
        tags.append(kind)
        if kind == b'IHDR':
            require(payload == struct.pack('>IIBBBBB', WIDTH, HEIGHT, depth, 6, 0, 0, 0), 'unexpected PNG dimensions/depth/format')
        elif kind == b'IDAT':
            compressed.extend(payload)
        elif kind == b'sRGB':
            require(payload == b'\x01', 'unexpected sRGB intent')
        elif kind == b'IEND':
            require(size == 0 and end + 4 == len(data), 'invalid PNG ending')
        else:
            require(kind[0] & 32, 'unsupported critical PNG chunk')
        offset = end + 4
    require(tags[0] == b'IHDR' and tags[-1] == b'IEND', 'missing PNG header/end')
    require(tags.count(b'IHDR') == tags.count(b'IEND') == tags.count(b'sRGB') == 1, 'missing/duplicate PNG metadata')
    bpp = 4 * (depth // 8)
    stride = WIDTH * bpp
    expected_size = (stride + 1) * HEIGHT
    decoder = zlib.decompressobj()
    filtered = decoder.decompress(compressed, expected_size + 1)
    require(len(filtered) == expected_size and decoder.eof and not decoder.unused_data, 'PNG inflated length mismatch')
    previous = bytearray(stride)
    pixels = bytearray()
    for y in range(HEIGHT):
        start = y * (stride + 1)
        kind = filtered[start]
        row = bytearray(filtered[start + 1:start + stride + 1])
        for i in range(stride):
            left = row[i - bpp] if i >= bpp else 0
            corner = previous[i - bpp] if i >= bpp else 0
            row[i] = (row[i] + predictor(kind, left, previous[i], corner)) & 255
        pixels.extend(row)
        previous = row
    return bytes(pixels)


def linear(v):
    s = v / 255
    return s / 12.92 if s <= .04045 else ((s + .055) / 1.055) ** 2.4


def over(fg, bg):
    a, b = fg[3] / 255, bg[3] / 255
    return [linear(fg[c]) * a + linear(bg[c]) * b * (1 - a) for c in range(3)] + [a + b * (1 - a)]


def encode(p):
    alpha = p[3]
    rgb = [v / alpha if alpha else 0 for v in p[:3]]
    srgb = [v * 12.92 if v <= .0031308 else 1.055 * v ** (1 / 2.4) - .055 for v in rgb]
    return [math.floor(min(1, max(0, v)) * 65535 + .5) for v in srgb + [alpha]]


def audit(directory, source_revision=None):
    report = json.loads((directory / 'report.json').read_text())
    require(report['schema_version'] == 2, 'requires schema 2 raw/16-bit report')
    require((report['width'], report['height'], report['input_bit_depth'], report['output_bit_depth']) == (WIDTH, HEIGHT, 8, 16), 'report format mismatch')
    require(report['linear_absolute_error_limit'] == .002 and report['png16_egress_code_value_error_limit'] == 2, 'changed numerical bounds')
    require(report['linear_readback_layout'] == {
        'format': 'IEEE 754 binary16', 'byte_order': 'little-endian', 'channels': 'RGBA',
        'order': 'row-major, top-to-bottom, left-to-right', 'bytes_per_pixel': 8,
        'row_stride_bytes': WIDTH * 8, 'padding_bytes': 0}, 'raw layout mismatch')
    source_checks = {}
    root = Path(__file__).resolve().parents[1]
    for key, relative in [('shader_sha256', 'crates/rezie-gpu/src/pool/colour.wgsl'),
                          ('probe_source_sha256', 'crates/rezie-gpu/src/pool/colour.rs'),
                          ('checker_source_sha256', 'crates/rezie-gpu/src/bin/rezie-colour-check.rs')]:
        source = (subprocess.check_output(['git', 'show', source_revision + ':' + relative], cwd=root)
                  if source_revision else (root / relative).read_bytes()).replace(b'\r\n', b'\n')
        matches = [name for name, value in [('LF', source), ('CRLF', source.replace(b'\n', b'\r\n'))] if digest(value) == report[key]]
        require(bool(matches), f'{relative}: measured source differs from current checkout')
        source_checks[relative] = matches[0]
    source = decode_png((directory / 'input-alpha.png').read_bytes(), 8)
    require(digest(source) == report['input_rgba_sha256'] == '47f24b377b54f5ea21902be325f7121f26ea3bcef70b9357e255a38e70dd3dad', 'scene input changed')
    require(len(report['cases']) == 5 and {c['name'] for c in report['cases']} == set(BACKGROUNDS), 'case inventory mismatch')
    cases = []
    for case in report['cases']:
        name = case['name']
        require(case['background_srgb_rgba'] == BACKGROUNDS[name], f'{name}: changed background')
        raw_info = case['linear_readback']
        require(raw_info['path'] == name + '.rgba16f.le', f'{name}: unexpected raw path')
        raw = (directory / raw_info['path']).read_bytes()
        require(len(raw) == raw_info['bytes'] == WIDTH * HEIGHT * 8, f'{name}: raw length mismatch')
        require(raw_info['pixels'] == case['pixels_checked'] == WIDTH * HEIGHT, f'{name}: wrong sample count')
        require(digest(raw) == raw_info['sha256'], f'{name}: raw hash mismatch')
        png = (directory / (name + '.png')).read_bytes()
        pixels = decode_png(png, 16)
        require(digest(png) == case['png_file_sha256'] and digest(pixels) == case['png_rgba16_be_sha256'], f'{name}: PNG hash mismatch')
        max_linear, max_egress, max_ideal, failing = 0., 0, 0, 0
        for index, (observed, actual) in enumerate(zip(struct.iter_unpack('<4e', raw), struct.iter_unpack('>4H', pixels))):
            require(all(math.isfinite(v) for v in observed), f'{name}: nonfinite raw sample {index}')
            target = over(source[index * 4:index * 4 + 4], BACKGROUNDS[name])
            error = max(abs(a - b) for a, b in zip(observed, target))
            egress = max(abs(a - b) for a, b in zip(actual, encode(observed)))
            ideal = max(abs(a - b) for a, b in zip(actual, encode(target)))
            max_linear = max(max_linear, error)
            max_egress = max(max_egress, egress)
            max_ideal = max(max_ideal, ideal)
            failing += error > .002 or egress > 2
        require(math.isclose(max_linear, case['max_linear_absolute_error'], abs_tol=1e-12, rel_tol=0), f'{name}: linear maximum cannot be reproduced')
        require((max_egress, max_ideal, failing) == (case['max_png16_egress_code_value_error'], case['max_png16_vs_ideal_code_value_error'], case['failing_pixels']), f'{name}: exported metrics cannot be reproduced')
        require(failing == 0 and case['first_failure'] is None, f'{name}: numerical bounds failed')
        cases.append({'name': name, 'pixels': WIDTH * HEIGHT, 'max_linear_absolute_error': max_linear,
                      'max_png16_egress_code_value_error': max_egress, 'max_png16_vs_ideal_code_value_error': max_ideal,
                      'png_file_sha256': digest(png), 'raw_file_sha256': digest(raw)})
    return {'ruling': 'PASS: independently reconstructed linear and PNG16 numerical checks',
            'adapter': report['adapter'], 'backend': report['backend'], 'source_line_endings': source_checks,
            'report_sha256': digest((directory / 'report.json').read_bytes()),
            'auditor_sha256': digest(Path(__file__).read_bytes()), 'cases': cases,
            'golden_approval_or_phase_gate': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--source-revision', help='explicit historical git revision of measured code')
    parser.add_argument('--output', type=Path, help='optional NEW audit JSON path')
    args = parser.parse_args()
    result = audit(args.directory, args.source_revision)
    text = json.dumps(result, indent=2) + '\n'
    if args.output:
        with args.output.open('x') as output:
            output.write(text)
    print(text, end='')


if __name__ == '__main__':
    main()
