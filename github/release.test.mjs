import test from 'node:test';
import assert from 'node:assert/strict';
import { nextVersion, releaseFiles } from '../scripts/prepare-release.mjs';

test('patch follows highest numeric stable tag, ignoring prereleases', () => {
  assert.equal(nextVersion(['v0.2.9', 'v0.2.10', 'v1.0.0-rc.1'], '0.2.0'), '0.2.11');
  assert.equal(nextVersion(['v0.2.0'], '0.2.0'), '0.2.1');
});
test('explicit newer versions and first release are preserved', () => {
  assert.equal(nextVersion(['v0.2.0'], '0.3.0'), '0.3.0');
  assert.equal(nextVersion([], '0.1.0'), '0.1.0');
  assert.throws(() => nextVersion([], '0.3.0-rc.1'), /stable semver/);
});
test('metadata aligns package and action, preserving dependency versions', () => {
  const files = releaseFiles('version = "0.2.0"\nclap = { version = "4" }', '    default: v0.2.0\n    default: text', '0.2.1');
  assert.equal(files.cargo, 'version = "0.2.1"\nclap = { version = "4" }');
  assert.equal(files.action, '    default: v0.2.1\n    default: text');
  assert.throws(() => releaseFiles('', '', '0.2.1'), /Missing/);
});

test('release pushes aligned metadata only to a tag and reuses it on retry', async () => {
  const { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } = await import('node:fs');
  const { tmpdir } = await import('node:os');
  const { execFileSync } = await import('node:child_process');
  const root = mkdtempSync(`${tmpdir()}/charter-release-test-`);
  const repo = `${root}/repo`;
  const git = (...args) => execFileSync('git', args, { cwd: repo, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
  try {
    mkdirSync(repo);
    git('init', '--bare', `${root}/remote.git`);
    git('init'); git('config', 'user.name', 'Release test'); git('config', 'user.email', 'release@example.invalid');
    writeFileSync(`${repo}/Cargo.toml`, '[package]\nname = "charter-cli"\nversion = "0.2.0"\nedition = "2024"\n');
    mkdirSync(`${repo}/src`); writeFileSync(`${repo}/src/lib.rs`, '');
    writeFileSync(`${repo}/action.yml`, '    default: v0.2.0\n');
    execFileSync('cargo', ['generate-lockfile', '--offline'], { cwd: repo });
    git('add', '.'); git('commit', '-m', 'tested source'); git('tag', 'v0.2.0');
    const source = git('rev-parse', 'HEAD');
    git('remote', 'add', 'origin', `${root}/remote.git`); git('push', 'origin', 'HEAD:main', '--tags');
    const env = { ...process.env, GITHUB_OUTPUT: `${root}/output`, GITHUB_STEP_SUMMARY: `${root}/summary` };
    const prepare = () => execFileSync(process.execPath, [new URL('../scripts/prepare-release.mjs', import.meta.url).pathname], { cwd: repo, env });
    prepare();
    assert.equal(git('rev-parse', 'v0.2.1^'), source);
    assert.match(readFileSync(`${repo}/Cargo.lock`, 'utf8'), /name = "charter-cli"\nversion = "0.2.1"/);
    assert.match(readFileSync(`${repo}/action.yml`, 'utf8'), /default: v0.2.1/);
    assert.equal(git('ls-remote', 'origin', 'refs/heads/main').split('\t')[0], source);
    git('checkout', '--detach', source); prepare();
    assert.equal(readFileSync(`${root}/output`, 'utf8'), 'tag=v0.2.1\ntag=v0.2.1\n');
    assert.equal(git('tag', '--list'), 'v0.2.0\nv0.2.1');
  } finally { rmSync(root, { recursive: true, force: true }); }
});
