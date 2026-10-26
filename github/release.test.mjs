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
