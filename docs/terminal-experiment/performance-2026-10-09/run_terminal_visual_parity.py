#!/usr/bin/env python3
"""Prepare fresh archived fixtures, then verify the existing visual parity gates."""
from pathlib import Path
import argparse
import datetime
import hashlib
import importlib.util
import json
import os
import re
import shutil
import stat
import subprocess
import sys

PARENT = Path('/private/tmp/potyi-terminal-performance-20261009-111723')
ARCHIVE = Path('/private/tmp/potyi-terminal-visual')
EVIDENCE = PARENT / 'baseline/docs/terminal-experiment/final-local-evidence/visual'
MAIN_TEST = 'app::input::tests::terminal_visual_probe::capture_terminal_visual_frames'
CLICK_TEST = 'app::input::tests::terminal_visual_probe::experimental_terminal_accent_filename_click_preserves_legacy_hit'
GIT_COMMITS = ['a6a8bb5592c859a5e56ac38b99f75fd0d15cd29e', '5fbf44f1b6d113f53a3c084cbf741e0146f5a3bf']


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + '\n')


def snapshot(root):
    entries = []
    for path in [root, *sorted(root.rglob('*'))]:
        info = path.lstat()
        require(not path.is_symlink(), f'Unexpected fixture symlink: {path}')
        item = {'path': path.relative_to(root).as_posix(),
                'kind': 'directory' if path.is_dir() else 'file',
                'mode': stat.S_IMODE(info.st_mode), 'mtime_ns': info.st_mtime_ns,
                'bytes': info.st_size}
        if path.is_file():
            item['sha256'] = digest(path)
        entries.append(item)
    return entries


def git(root, arguments, environment=None):
    env = dict(os.environ, GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL='/dev/null')
    if environment:
        env.update(environment)
    command = ['git', '-c', 'commit.gpgsign=false', '-c', 'core.hooksPath=/dev/null',
               '-c', 'core.autocrlf=false', *arguments]
    result = subprocess.run(command, cwd=root, env=env, capture_output=True, text=True, check=True)
    return result.stdout.strip()


def prepare(work):
    manifest_path = work / 'fixture-provenance.json'
    if work.exists():
        require(manifest_path.is_file(), f'Refusing an unrecognized existing directory: {work}')
        manifest = json.loads(manifest_path.read_text())
        verify_fixtures(work, manifest)
        return manifest
    templates = {name: ARCHIVE / source for name, source in
                 [('common', 'fixture'), ('leading', 'leading-fixture')]}
    before = {name: snapshot(root) for name, root in templates.items()}
    historical = [ARCHIVE / filename for filename in
                  ['run-performance-final-v2.py', 'compare_frames.py', 'finalize_evidence.py']]
    historical += [EVIDENCE / name / 'capture-provenance.json' for name in ['common', 'leading', 'quick-open']]
    inputs = {str(path): digest(path) for path in historical}
    expected = {}
    for name in ['common', 'leading', 'quick-open']:
        prior = json.loads((EVIDENCE / name / 'capture-provenance.json').read_text())
        expected[name] = {group: sorted(Path(item['path']).name for item in prior[group]
                                       if Path(item['path']).parts[0] == 'old')
                          for group in ['frames', 'clipboard']}
    for name in ['common', 'leading']:
        require(len(expected[name]['frames']) == 27 and len(expected[name]['clipboard']) == 10,
                f'Archived expected gate counts differ: {name}')
    require(expected['quick-open'] == {'frames': ['accent-filename-before.bmp'],
                                     'clipboard': ['accent-filename-before.txt']}, 'Archived click gate differs')
    work.mkdir(parents=True)
    fixture_parent = work / 'fixtures'
    fixture_parent.mkdir()
    for name, root in templates.items():
        destination = fixture_parent / name
        shutil.copytree(root, destination, copy_function=shutil.copy2)
        require(snapshot(destination) == before[name], f'Fresh fixture metadata/content differ: {name}')
    git_root = fixture_parent / 'git'
    git_root.mkdir()
    git(git_root, ['init', '--quiet', '--object-format=sha1', '--initial-branch=feature/terminal-visual-fixture'])
    git(git_root, ['config', 'core.ignorecase', 'true'])
    git(git_root, ['config', 'core.precomposeunicode', 'true'])
    identities = {'GIT_AUTHOR_NAME': 'Potyi Visual', 'GIT_AUTHOR_EMAIL': 'visual@example.invalid',
                  'GIT_COMMITTER_NAME': 'Potyi Visual', 'GIT_COMMITTER_EMAIL': 'visual@example.invalid'}
    commits = []
    for content, message, date in [(b'first line\n', 'First fixture commit', '2020-01-02T03:04:05+0000'),
                                   (b'changed line\nadded line\n', 'Change fixture note', '2020-01-03T04:05:06+0000')]:
        (git_root / 'note.txt').write_bytes(content)
        git(git_root, ['add', '--', 'note.txt'])
        git(git_root, ['commit', '--quiet', '-m', message],
            dict(identities, GIT_AUTHOR_DATE=date, GIT_COMMITTER_DATE=date))
        commits.append(git(git_root, ['rev-parse', 'HEAD']))
    require(commits == GIT_COMMITS, f'Deterministic Git commits differ: {commits}')
    require(not git(git_root, ['status', '--porcelain=v1']), 'Git fixture must be clean')
    note_stat = (ARCHIVE / 'git-fixture/note.txt').stat()
    os.utime(git_root / 'note.txt', ns=(note_stat.st_atime_ns, note_stat.st_mtime_ns))
    after = {name: snapshot(root) for name, root in templates.items()}
    require(before == after, 'Archived templates changed while being copied')
    require(all(digest(Path(path)) == value for path, value in inputs.items()), 'Archived evidence changed')
    fixtures = {name: snapshot(fixture_parent / name) for name in ['common', 'leading', 'git']}
    manifest = {'prepared_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'template_paths': {name: str(path) for name, path in templates.items()},
                'templates_before': before, 'templates_after': after,
                'historical_readonly_sha256': inputs, 'fixtures': fixtures,
                'deterministic_git_commits': commits, 'expected_files': expected,
                'fixture_algorithm': 'Sorted relative paths with lstat mode, size, mtime_ns and file SHA256; no atime.',
                'notes': ['Fresh common/leading copies preserve filenames, bytes, directory geometry, modes and timestamps.',
                          'Git uses the exact archived trees, author, committer, messages and timestamps. Branch uses feature/ prefix.',
                          'Captured path text changes with fresh fixture root; compare old/new in the same fresh root, not historical image bytes.']}
    write_json(manifest_path, manifest)
    return manifest


def verify_fixtures(work, manifest):
    for name, expected in manifest['fixtures'].items():
        require(snapshot(work / 'fixtures' / name) == expected, f'Prepared fixture changed: {name}')
    require(all(digest(Path(path)) == value for path, value in manifest['historical_readonly_sha256'].items()),
            'Historical evidence inputs changed')


def identity(args, runner):
    source = runner.source_provenance(args.candidate)
    binary_hash, helper_hash = digest(args.binary), digest(args.helper)
    attestation = runner.validated_build_provenance(args.build_provenance, source, binary_hash,
                                                  args.helper.resolve(), helper_hash)
    return {'observed_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
            'candidate': str(args.candidate), 'source_sha256': source['source_sha256'],
            'test_binary': str(args.binary), 'test_binary_sha256': binary_hash,
            'helper': str(args.helper), 'helper_sha256': helper_hash,
            'build_provenance': attestation, 'runner_sha256': digest(Path(__file__))}


def same_identity(before, after):
    for key in ['source_sha256', 'test_binary_sha256', 'helper_sha256', 'runner_sha256']:
        require(before[key] == after[key], f'Capture identity changed: {key}')
    require(before['build_provenance'] == after['build_provenance'], 'Build attestation changed')


def case(args, runner, work, manifest, output, fixture, kind, test):
    destination = output / kind
    destination.mkdir(parents=True)
    verify_fixtures(work, manifest)
    before = identity(args, runner)
    environment = dict(os.environ, SDL_VIDEODRIVER='dummy', SDL_RENDER_DRIVER='software',
                       POTYI_TERM_TEST_CLIENT=str(args.helper), POTYI_TERM_VISUAL_KIND=kind,
                       POTYI_TERM_VISUAL_ROOT=str(work / 'fixtures' / fixture),
                       POTYI_TERM_VISUAL_GIT_ROOT=str(work / 'fixtures/git'),
                       POTYI_TERM_VISUAL_OUTPUT=str(destination))
    command = [str(args.binary), '--exact', test, '--ignored', '--nocapture', '--test-threads=1']
    print(f'Capturing {output.name} {kind} with explicit immutable helper', flush=True)
    log_path = output / (kind + '.log')
    with log_path.open('w') as log:
        result = subprocess.run(command, cwd=args.candidate, env=environment,
                                stdout=log, stderr=subprocess.STDOUT, timeout=180)
    after = identity(args, runner)
    same_identity(before, after)
    verify_fixtures(work, manifest)
    write_json(output / (kind + '-provenance.json'), {'before': before, 'after': after,
               'command': command, 'exit_code': result.returncode, 'fixture': fixture,
               'fixture_manifest_sha256': digest(work / 'fixture-provenance.json'),
               'log_sha256': digest(log_path)})
    text = log_path.read_text()
    require(result.returncode == 0 and re.search(r'test result: ok\. 1 passed; 0 failed;', text),
            f'Exactly one successful test required; see {log_path}')
    if test == CLICK_TEST:
        require('POTYI_ACCENT_CLICK ' in text and 'actual=Some(' in text, 'Actual filename click evidence missing')
    return before, after


def compare(output, expected):
    from PIL import Image, ImageChops, ImageDraw
    for kind in ['old', 'new']:
        for group, suffix in [('frames', '*.bmp'), ('clipboard', '*.txt')]:
            actual = sorted(path.name for path in (output / kind).glob(suffix))
            require(actual == expected[group], f'Unexpected {kind} {group} names/count in {output}: {actual}')
    frames, copies = [], []
    for name in expected['frames']:
        old = Image.open(output / 'old' / name).convert('RGB')
        new = Image.open(output / 'new' / name).convert('RGB')
        row = {'scene': Path(name).stem, 'size_old': old.size, 'size_new': new.size,
               'rgb_sha256_old': hashlib.sha256(old.tobytes()).hexdigest(),
               'rgb_sha256_new': hashlib.sha256(new.tobytes()).hexdigest()}
        if old.size == new.size:
            difference = ImageChops.difference(old, new)
            row.update(changed_pixels=sum(pixel != (0, 0, 0) for pixel in difference.getdata()),
                       bounds=difference.getbbox())
        gallery = Image.new('RGB', (old.width + new.width, max(old.height, new.height) + 24), (35, 35, 35))
        draw = ImageDraw.Draw(gallery)
        draw.text((8, 5), 'original :term', fill='white')
        draw.text((old.width + 8, 5), 'new integration', fill='white')
        gallery.paste(old, (0, 24))
        gallery.paste(new, (old.width, 24))
        gallery.save(output / (Path(name).stem + '-compare.png'))
        frames.append(row)
    for name in expected['clipboard']:
        old, new = (output / 'old' / name).read_bytes(), (output / 'new' / name).read_bytes()
        row = {'scene': Path(name).stem, 'equal': old == new, 'bytes_old': len(old), 'bytes_new': len(new),
               'sha256_old': hashlib.sha256(old).hexdigest(), 'sha256_new': hashlib.sha256(new).hexdigest()}
        if old != new:
            position = next((index for index, (a, b) in enumerate(zip(old, new)) if a != b), min(len(old), len(new)))
            row.update(first_difference=position, old_context=repr(old[max(0, position-40):position+100]),
                       new_context=repr(new[max(0, position-40):position+100]))
        copies.append(row)
    report = {'frames': frames, 'clipboard': copies}
    write_json(output / 'comparison.json', report)
    require(all(row.get('changed_pixels') == 0 for row in frames), f'RGB visual mismatch: {output}/comparison.json')
    require(all(row['equal'] for row in copies), f'Clipboard mismatch: {output}/comparison.json')
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work-root', type=Path, default=PARENT / 'review/visual-parity')
    parser.add_argument('--candidate', type=Path, default=PARENT / 'candidate')
    parser.add_argument('--binary', type=Path)
    parser.add_argument('--helper', type=Path)
    parser.add_argument('--build-provenance', type=Path)
    parser.add_argument('--prepare-only', action='store_true')
    args = parser.parse_args()
    args.work_root, args.candidate = args.work_root.resolve(), args.candidate.resolve()
    manifest = prepare(args.work_root)
    if args.prepare_only:
        print(json.dumps({'prepared': str(args.work_root), 'fixture_manifest_sha256': digest(args.work_root / 'fixture-provenance.json'),
                          'git_commits': manifest['deterministic_git_commits'], 'tests_launched': False}))
        return
    require(args.binary and args.helper and args.build_provenance, 'Capture requires --binary, --helper, --build-provenance')
    args.binary, args.helper, args.build_provenance = args.binary.resolve(), args.helper.resolve(), args.build_provenance.resolve()
    spec = importlib.util.spec_from_file_location('terminal_resource_runner', args.candidate / 'tools/qa/terminal-resources.py')
    runner = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = runner
    spec.loader.exec_module(runner)
    capture = args.work_root / 'captures'
    require(not capture.exists(), f'Refusing to overwrite previous or partial capture: {capture}')
    capture.mkdir()
    before = identity(args, runner)
    results = {}
    for name, fixture, test in [('common', 'common', MAIN_TEST), ('leading', 'leading', MAIN_TEST),
                               ('quick-open', 'leading', CLICK_TEST)]:
        output = capture / name
        output.mkdir()
        case_before = identity(args, runner)
        for kind in ['old', 'new']:
            case(args, runner, args.work_root, manifest, output, fixture, kind, test)
        report = compare(output, manifest['expected_files'][name])
        case_after = identity(args, runner)
        same_identity(case_before, case_after)
        artifacts = {path.relative_to(output).as_posix(): digest(path) for path in sorted(output.rglob('*')) if path.is_file()}
        write_json(output / 'capture-provenance.json', {'before': case_before, 'after': case_after,
                   'fixture_manifest_sha256': digest(args.work_root / 'fixture-provenance.json'),
                   'artifacts_sha256': artifacts,
                   'frames': [{'path': path.relative_to(output).as_posix(), 'sha256': digest(path)} for path in sorted(output.glob('*/*.bmp'))],
                   'clipboard': [{'path': path.relative_to(output).as_posix(), 'sha256': digest(path)} for path in sorted(output.glob('*/*.txt'))]})
        results[name] = {'scene_count': len(report['frames']), 'exact_frame_count': len(report['frames']),
                         'clipboard_count': len(report['clipboard']), 'exact_clipboard_count': len(report['clipboard']),
                         'capture_provenance_sha256': digest(output / 'capture-provenance.json')}
        print(f'{name}: {len(report["frames"])} exact RGB frames; {len(report["clipboard"])} exact clipboard files', flush=True)
    after = identity(args, runner)
    same_identity(before, after)
    verify_fixtures(args.work_root, manifest)
    write_json(capture / 'summary.json', {'before': before, 'after': after, 'results': results,
               'limitations': ['Headless SDL dummy software renderer and embedded DejaVu Sans Mono18; no native compositor or other font/theme/DPI guarantee.',
                               'Fresh fixture root paths differ from historical evidence; only matched current original/new RGB and exact clipboard bytes are compared.',
                               'Native Windows is excluded by the existing cfg(unix) visual probes.',
                               'This script verifies frozen release provenance and capture input stability; root remains responsible for performing the attested build.']})
    print('Both 27-frame/10-clipboard comparisons and original/new actual accent filename clicks passed.', flush=True)


if __name__ == '__main__':
    main()
