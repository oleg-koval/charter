import { spawnSync } from 'node:child_process';
import { appendFileSync } from 'node:fs';
import { annotation, flightReport } from './report.mjs';

function command(args) {
  const result = spawnSync('charter', [...args, '--dir', process.env.CHARTER_DIR || '.charter', '--json'], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'inherit'] });
  try {
    const data = JSON.parse(result.stdout);
    if (typeof data?.ok !== 'boolean') throw new Error('missing ok verdict');
    if (args[0] === 'doctor' ? !Array.isArray(data.gaps) || !Array.isArray(data.waived)
      : !Array.isArray(data.lines)) throw new Error('missing finding arrays');
    return { data, status: result.status, error: result.error?.message };
  } catch {
    return { data: {}, status: result.status, error: result.error?.message || 'Charter returned no valid result; inspect this step’s logs.' };
  }
}

function station(name, result) {
  if (result.error) return { name, state: 'INFRA ERROR', detail: result.error, issues: [result.error] };
  const gaps = Array.isArray(result.data.gaps) ? result.data.gaps : [];
  const lines = Array.isArray(result.data.lines) ? result.data.lines : [];
  const issues = [...gaps.map((gap) => `${gap.id}: ${gap.message} (${gap.file})`), ...lines];
  const pass = result.status === 0 && result.data.ok && issues.length === 0;
  return { name, state: pass ? 'PASS' : 'FAIL', detail: pass ? 'Validated in this run' : 'Resolve the items below',
    issues: pass ? [] : issues.length > 0 ? issues : ['Charter failed without finding details; inspect logs.'] };
}

const doctor = !['true', 'false'].includes(process.env.CHARTER_CHECKS)
  ? { data: {}, error: 'checks must be true or false' }
  : process.env.CHARTER_SETUP === 'success' ? command(['doctor'])
  : { data: {}, error: 'Charter installation did not succeed; inspect the install step.' };
const policy = station('Policy mapping', doctor);
const verify = (name, args) => policy.state === 'PASS' ? station(name, command(args)) : {
  name, state: 'NOT RUN', detail: 'Policy or installation failed', issues: ['Fix the earlier failure before retrying.'],
};
const stages = [policy,
  ...(process.env.CHARTER_CHECKS === 'true' ? [verify('Mapped code checks', ['check'])] : []),
  ...(process.env.CHARTER_PR_BODY_FILE ? [verify('PR sections', ['verify-pr', process.env.CHARTER_PR_BODY_FILE])] : []),
];
const waived = Array.isArray(doctor.data.waived) ? doctor.data.waived : null;
const url = `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}`;
const report = flightReport({ scope: 'Selected stations only',
  sha: process.env.CHARTER_HEAD_SHA || process.env.GITHUB_SHA, url, stages, waived });
appendFileSync(process.env.GITHUB_STEP_SUMMARY, report);
for (const stage of stages) {
  for (const issue of stage.issues) console.log(annotation('error',
    stage.state === 'INFRA ERROR' ? 'Charter infrastructure' : 'Charter finding', `${stage.name}: ${issue}`));
}
for (const waiver of waived || []) console.log(annotation('notice', 'Charter waiver', `${waiver.rule}: ${waiver.reason}`));
console.log(report);
process.exitCode = stages.every((stage) => stage.state === 'PASS') ? 0 : 1;
