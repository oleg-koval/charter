import { flightReport } from './report.mjs';

const marker = '<!-- charter-flight-report -->';

async function inspection(github, repo, pr, [workflow_id, name]) {
  const { data } = await github.rest.actions.listWorkflowRuns({ ...repo, workflow_id,
    head_sha: pr.head.sha, event: 'pull_request', per_page: 100 });
  const run = data.workflow_runs.find((item) => item.head_branch === pr.head.ref
    && item.head_repository?.id === pr.head.repo.id && item.created_at >= pr.created_at);
  if (!run || run.status !== 'completed') return { name, run, annotations: [] };
  const jobs = await github.paginate(github.rest.actions.listJobsForWorkflowRun, { ...repo, run_id: run.id });
  const annotations = await Promise.all(jobs.map((job) => github.paginate(
    github.rest.checks.listAnnotations, { ...repo, check_run_id: Number(job.check_run_url.split('/').at(-1)) })));
  return { name, run, annotations: annotations.flat() };
}

export function station({ name, run, annotations }) {
  if (!run) return { name, state: 'NOT RUN', detail: 'No run found for this PR head', issues: ['Wait for a run, or inspect workflow triggers.'] };
  if (run.status !== 'completed') return { name, state: 'PENDING', detail: run.status, issues: [] };
  if (run.conclusion === 'cancelled') return { name, state: 'CANCELLED', detail: `Run ${run.id} was cancelled`, issues: ['Wait for the replacement run or rerun the check.'] };
  const state = run.conclusion === 'success' ? 'PASS' : annotations.some((item) => item.title === 'Charter finding')
    && !annotations.some((item) => item.title === 'Charter infrastructure') ? 'FAIL' : 'INFRA ERROR';
  const findings = annotations.filter((item) => item.annotation_level === 'failure').map((item) => item.message);
  return { name, state, detail: `Run ${run.id}: ${run.conclusion}`,
    issues: state === 'PASS' ? [] : findings.length > 0 ? findings.slice(0, 10) : [`${run.conclusion}; inspect failed steps in this run.`] };
}

function report(pr, inspections) {
  const waivers = inspections.flatMap((item) => item.annotations)
    .filter((item) => item.title === 'Charter waiver').map((item) => ({
      rule: item.message.split(':')[0], reason: item.message.split(':').slice(1).join(':').trim(),
    }));
  const unique = waivers.filter((item, index) => waivers.findIndex((other) => other.rule === item.rule) === index);
  const text = flightReport({ scope: 'PR event snapshots', sha: pr.head.sha, url: pr.html_url + '/checks',
    stages: inspections.map(station), waived: unique.length > 0 ? unique : null });
  const links = inspections.filter((item) => item.run)
    .map((item) => `[${item.name} · run ${item.run.id}](${item.run.html_url})`).join(' · ');
  return `${marker}\n${text.slice(0, 55000)}\n${text.length > 55000 ? 'Report truncated; open the runs for all findings.\n' : ''}${links}\n\nUpdated from GitHub check results. PR edits need a fresh PR-sections run; these results do not enforce branch protection.`;
}

export async function publish({ github, context, core }) {
  const source = context.payload.workflow_run;
  if (!source) throw new Error('The report action requires a workflow_run event');
  const workflows = [[process.env.CHARTER_CODE_WORKFLOW || 'charter.yml', 'Code checks'],
    [process.env.CHARTER_PR_WORKFLOW || 'charter-pr.yml', 'PR sections']];
  if (source.event !== 'pull_request' || !workflows.some(([file]) => source.path?.split('@')[0] === `.github/workflows/${file}`))
    return core.notice('Run is outside the configured PR workflows; report intentionally skipped.');
  const repo = context.repo;
  const pulls = await github.paginate(github.rest.pulls.list, { ...repo, state: 'open',
    head: `${source.head_repository.owner.login}:${source.head_branch}` });
  const pr = pulls.find((item) => item.head.sha === source.head_sha && item.head.repo?.id === source.head_repository.id);
  if (!pr) return core.notice('No current open PR matches this run; report intentionally skipped.');
  const inspections = await Promise.all(workflows.map((workflow) => inspection(github, repo, pr, workflow)));
  const comments = await github.paginate(github.rest.issues.listComments, { ...repo, issue_number: pr.number });
  const existing = comments.find((item) => item.user?.login === 'github-actions[bot]' && item.body?.startsWith(marker));
  const { data: current } = await github.rest.pulls.get({ ...repo, pull_number: pr.number });
  if (current.state !== 'open' || current.head.sha !== pr.head.sha || current.updated_at !== pr.updated_at)
    return core.notice('PR changed while collecting results; report intentionally skipped.');
  const body = report(pr, inspections);
  if (existing) await github.rest.issues.updateComment({ ...repo, comment_id: existing.id, body });
  else await github.rest.issues.createComment({ ...repo, issue_number: pr.number, body });
}
