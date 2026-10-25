import { test, mock } from 'node:test';
import assert from 'node:assert/strict';
import { publish, station } from './publish.mjs';

function fixture({ existing = false, stale = false, pending = false, unrelated = false, missing = false } = {}) {
  const pr = { number: 7, state: 'open', created_at: '2026-01-01', updated_at: '2026-01-03',
    html_url: 'https://github.com/owner/repo/pull/7', head: { sha: 'abc', ref: 'feature', repo: { id: 1 } } };
  const run = { id: 11, head_sha: 'abc', head_branch: 'feature', head_repository: { id: 1 },
    created_at: '2026-01-02', status: pending ? 'queued' : 'completed', conclusion: 'failure', html_url: 'https://github.com/owner/repo/actions/runs/11' };
  const createComment = mock.fn(async () => ({}));
  const updateComment = mock.fn(async () => ({}));
  const github = { rest: {
    pulls: { list: async () => ({ data: [pr] }), get: async () => ({ data: stale ? { ...pr, head: { ...pr.head, sha: 'new' } } : pr }) },
    actions: { listWorkflowRuns: async () => { if (missing) throw Object.assign(new Error('Not Found'), { status: 404 }); return { data: { workflow_runs: [run] } }; },
      listJobsForWorkflowRun: async () => ({ data: [{ check_run_url: 'https://api.github.com/repos/owner/repo/check-runs/11' }] }) },
    checks: { listAnnotations: async () => ({ data: [
      { title: 'Charter finding', annotation_level: 'failure', message: 'Problem heading is missing | @owner' },
      { title: 'Charter waiver', annotation_level: 'notice', message: 'C2: No cast lint' },
    ] }) },
    issues: { listComments: async () => ({ data: existing ? [
      { id: 99, user: { login: 'github-actions[bot]' }, body: '<!-- charter-flight-report -->old' },
    ] : [] }), createComment, updateComment },
  }, paginate: async (fn, args) => (await fn(args)).data };
  return { github, createComment, updateComment, core: { notice: mock.fn() }, context: {
    repo: { owner: 'owner', repo: 'repo' }, payload: { workflow_run: { event: 'pull_request', name: 'Charter', path: '.github/workflows/charter.yml',
      head_sha: 'abc', head_branch: 'feature', head_repository: { id: unrelated ? 2 : 1, owner: { login: 'owner' } } } },
  } };
}

test('creates one safe combined report from GitHub verdicts and annotations', async () => {
  const f = fixture();
  await publish(f);
  assert.equal(f.createComment.mock.callCount(), 1);
  const body = f.createComment.mock.calls[0].arguments[0].body;
  assert.match(body, /HOLD[\s\S]*Code checks[\s\S]*PR sections/);
  assert.match(body, /Problem heading is missing/);
  assert.doesNotMatch(body, /@owner/);
  assert.equal((body.match(/\*\*C2\*\*/g) || []).length, 1);
});

test('updates the bot-owned report instead of creating comment spam', async () => {
  const f = fixture({ existing: true });
  await publish(f);
  assert.equal(f.createComment.mock.callCount(), 0);
  assert.equal(f.updateComment.mock.calls[0].arguments[0].comment_id, 99);
});

for (const options of [{ stale: true }, { unrelated: true }]) {
  test(`skips stale heads and mismatched repositories: ${JSON.stringify(options)}`, async () => {
    const f = fixture(options);
    await publish(f);
    assert.equal(f.createComment.mock.callCount() + f.updateComment.mock.callCount(), 0);
  });
}

test('pending and unavailable results never become a pass', async () => {
  const f = fixture({ pending: true });
  await publish(f);
  assert.match(f.createComment.mock.calls[0].arguments[0].body, /HOLD[\s\S]*PENDING/);
  assert.match(f.createComment.mock.calls[0].arguments[0].body, /Waiver register · UNAVAILABLE/);
  assert.equal(station({ name: 'Code', annotations: [] }).state, 'NOT RUN');
  assert.equal(station({ name: 'Code', run: { status: 'completed', conclusion: 'cancelled' }, annotations: [] }).state, 'CANCELLED');
});

test('missing workflow stays visible as NOT RUN', async () => {
  const f = fixture({ missing: true });
  await publish(f);
  assert.match(f.createComment.mock.calls[0].arguments[0].body, /HOLD[\s\S]*NOT RUN/);
});
