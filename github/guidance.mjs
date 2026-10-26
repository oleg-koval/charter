const billingBlocked = (stage) => stage.state === 'INFRA ERROR' && stage.issues.some((issue) =>
  /job was not started because recent account payments have failed or your spending limit needs to be increased/i.test(issue));
const templates = [
  { matches: (stages) => stages.some(billingBlocked), title: 'GitHub blocked checks because of billing',
    reason: 'GitHub did not start the workflows marked as blocked below. Their code and PR checks have no result yet.',
    actions: ['Account owner: Open Billing & plans for the account paying for Actions and resolve the failed-payment or spending-limit warning.',
      'Agent or maintainer: After the owner confirms Actions is unblocked, rerun the affected workflows using the links or commands below.'],
    verify: 'Resolved when the blocked workflows start and finish. Any check failures they then report need their own fixes.' },
  { matches: (stages) => stages.some((s) => s.state === 'INFRA ERROR'), title: 'Checks could not complete',
    reason: 'A workflow or tool failed. The cause is not confirmed by this report.',
    actions: ['Agent or maintainer: Open the affected run below and inspect the first failed step and its error.',
      'Agent or maintainer: Fix the reported tool or workflow problem, then rerun that workflow. Ask the account owner if it requires billing or access changes.'],
    verify: 'Resolved when the affected checks finish and report their results.' },
  { matches: (stages) => stages.some((s) => s.state === 'FAIL' && s.name === 'Policy mapping'), title: 'Charter policy needs configuration',
    reason: 'Charter found gaps in the rule mappings. Checks depending on those mappings may not have run.',
    actions: ['Agent or maintainer: Open the configured policy directory and correct the mappings named in Policy configuration errors.',
      'Agent or maintainer: Run charter doctor with that policy directory, then rerun the affected checks.'],
    verify: 'Resolved when policy validation passes and the dependent checks finish.' },
  { matches: (stages) => stages.some((s) => s.state === 'FAIL')
      && stages.filter((s) => s.state === 'FAIL').every((s) => s.name === 'PR sections'), title: 'PR description needs changes',
    reason: 'Required sections are missing or empty in the PR description.',
    actions: ['PR author or agent: Edit the PR description and fill in each missing or empty heading listed below.',
      'Agent or maintainer: Wait for the new PR-description check triggered by that edit. Rerunning the old event checks its original description.'],
    verify: 'Resolved when the new PR-description check passes.' },
  { matches: (stages) => stages.some((s) => s.state === 'FAIL'), title: 'Changes are needed',
    reason: 'The checks ran and found the issues listed below.',
    actions: ['PR author or agent: For missing PR sections, edit the PR description and fill in the named headings. For code failures, open the failed run and fix the first failing command and its reported errors.',
      'Agent or maintainer: Push code fixes to trigger checks. PR-description fixes need a fresh edited event; rerunning an old event checks its original description.'],
    verify: 'Resolved when fresh checks pass for the current commit and PR description.' },
  { matches: (stages) => stages.some((s) => s.state === 'CANCELLED'), title: 'Checks were cancelled',
    reason: 'Cancelled workflows do not provide a completed result.',
    actions: ['Agent or maintainer: Check for a replacement run on this commit. If none exists, rerun the cancelled workflow.'],
    verify: 'Resolved when the replacement workflows finish.' },
  { matches: (stages) => stages.some((s) => s.state === 'NOT RUN') || stages.length === 0, title: 'Check results are missing',
    reason: 'At least one required workflow has no result for this commit.',
    actions: ['Agent or maintainer: Check the workflow links and trigger configuration. Ensure each selected workflow exists and receives the PR event.'],
    verify: 'Resolved when the missing workflows produce results for this commit.' },
  { matches: (stages) => stages.some((s) => s.state === 'PENDING'), title: 'Checks are still running',
    reason: 'Wait for the workflows below to finish.', actions: ['Agent or maintainer: If a job stays queued, check runner availability in the run.'],
    verify: 'This report updates after a workflow finishes.' },
];
export const guidance = (stages) => templates.find((item) => item.matches(stages)) || {
  title: 'Selected checks passed', reason: 'No failures were reported by these checks.', actions: [],
  verify: 'Other CI checks and human review still apply.',
};
export const resultText = (stage) => billingBlocked(stage) ? 'Blocked by GitHub billing' : ({
  PASS: 'Passed', FAIL: 'Failed', 'INFRA ERROR': 'Could not complete', 'NOT RUN': 'Not run', CANCELLED: 'Cancelled', PENDING: 'Waiting',
})[stage.state];
export const retryCommands = (stages, fallback) => [...new Set(stages
  .filter((stage) => ['FAIL', 'INFRA ERROR', 'CANCELLED'].includes(stage.state))
  .flatMap((stage) => {
    const match = (stage.url || fallback)?.match(/^https?:\/\/[^/]+\/([\w.-]+\/[\w.-]+)\/actions\/runs\/(\d+)$/);
    return match ? [`gh run rerun ${match[2]} --repo ${match[1]}${stage.state === 'CANCELLED' ? '' : ' --failed'}`] : [];
  }))];
