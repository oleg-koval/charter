import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, chmodSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { annotation, flightReport } from './report.mjs';

test('a passed station never hides waivers or implies full-contract clearance', () => {
  const report = flightReport({ scope: 'PR sections', sha: 'abc', url: 'https://github.com/run',
    stages: [{ name: 'PR sections', state: 'PASS', detail: 'Validated', issues: [] }],
    waived: [{ rule: 'C2', reason: 'No cast lint' }] });
  assert.match(report, /🟢 PASS/);
  assert.match(report, /C2.*No cast lint/);
  assert.match(report, /not merge clearance/);
});

test('findings cannot inject Markdown rows, mentions or runner commands', () => {
  const report = flightReport({ scope: 'PR', sha: 'abc', url: 'https://github.com/run',
    stages: [{ name: 'PR sections', state: 'FAIL', detail: 'Missing headings', issues: ['<b>@owner|[fake](url)\n::error::injected'] }] });
  assert.match(report, /🔴 HOLD/);
  assert.doesNotMatch(report, /<b>|@owner|\[fake\]|\n::error/);
  assert.equal(annotation('error', 'Charter finding', 'bad%\n::warning::x'), '::error title=Charter finding::bad%25%0A::warning::x');
});

for (const mode of ['missing', 'malformed', 'partial', 'findings', 'pass']) {
  test(`CLI keeps ${mode} distinct and writes a summary on failure`, () => {
    const dir = mkdtempSync(join(tmpdir(), 'charter-report-'));
    try {
      const summary = join(dir, 'summary.md');
      const event = join(dir, 'event.json');
      writeFileSync(event, JSON.stringify({ pull_request: { body: 'description', head: { sha: 'abc' } } }));
      if (mode !== 'missing') {
        const fake = join(dir, 'charter');
        writeFileSync(fake, `#!${process.execPath}\nconst doctor=process.argv.includes('doctor');\nconsole.log(process.env.MODE==='malformed'?'garbage':JSON.stringify(process.env.MODE==='partial'?{ok:true}:doctor?{ok:true,gaps:[],waived:[]}:{ok:process.env.MODE==='pass',lines:process.env.MODE==='pass'?[]:['Problem  artifact heading is missing']}));\nprocess.exitCode=!doctor&&process.env.MODE!=='pass'?1:0;\n`);
        chmodSync(fake, 0o755);
      }
      const result = spawnSync(process.execPath, ['github/run.mjs'], { encoding: 'utf8', env: {
        ...process.env, PATH: dir, MODE: mode, CHARTER_SETUP: 'success', RUNNER_TEMP: dir,
        CHARTER_PR_BODY_FILE: event, CHARTER_CHECKS: 'false', CHARTER_HEAD_SHA: 'abc', GITHUB_STEP_SUMMARY: summary,
        GITHUB_SERVER_URL: 'https://github.com', GITHUB_REPOSITORY: 'owner/repo', GITHUB_RUN_ID: '1',
      } });
      assert.equal(result.status, mode === 'pass' ? 0 : 1, result.stderr);
      const report = readFileSync(summary, 'utf8');
      assert.match(report, mode === 'pass' ? /🟢 PASS/ : /🔴 HOLD/);
      assert.doesNotMatch(report, /Mapped code checks/);
      if (mode === 'findings') assert.match(report, /Problem.*artifact heading is missing/);
      if (['missing', 'malformed', 'partial'].includes(mode)) assert.match(report, /INFRA ERROR[\s\S]*NOT RUN/);
    } finally { rmSync(dir, { recursive: true, force: true }); }
  });
}
