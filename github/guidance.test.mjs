import { test } from 'node:test';
import assert from 'node:assert/strict';
import { guidance, retryCommands } from './guidance.mjs';
import { flightReport } from './report.mjs';

const blocked = { name: 'Code checks', state: 'INFRA ERROR', detail: 'Run 40294743172: failure',
  url: 'https://github.com/oleg-koval/vlag/actions/runs/40294743172',
  issues: ["The job was not started because recent account payments have failed or your spending limit needs to be increased. Please check the 'Billing & plans' section in your settings"] };

test('billing report leads with cause, owned actions and recovery before technical evidence', () => {
  const report = flightReport({ scope: 'PR', sha: 'abc', url: 'https://github.com/oleg-koval/vlag/pull/7/checks',
    stages: [blocked, { ...blocked, name: 'PR sections', url: blocked.url.replace('40294743172', '40294772016') }] });
  const visible = report.split('<details>')[0];
  assert.match(visible, /GitHub blocked checks because of billing/);
  assert.match(visible, /Account owner:.*Billing & plans/);
  assert.match(visible, /Agent or maintainer: After the owner confirms Actions is unblocked, rerun/);
  assert.match(visible, /Resolved when the blocked workflows start and finish/);
  assert.doesNotMatch(visible, /INFRA ERROR|UNAVAILABLE|Run 4029|recent account payments/);
  assert.equal((visible.match(/Account owner:/g) || []).length, 1);
  assert.match(report, /gh run rerun 40294743172 --repo oleg-koval\/vlag --failed/);
  assert.match(report, /gh run rerun 40294772016 --repo oleg-koval\/vlag --failed/);
});

test('unknown infrastructure and mixed results preserve uncertainty and existing findings', () => {
  assert.equal(guidance([{ ...blocked, issues: ['Network unavailable'] }]).title, 'Checks could not complete');
  assert.equal(guidance([{ ...blocked, state: 'FAIL' }]).title, 'Changes are needed');
  const stages = [blocked, { name: 'PR sections', state: 'FAIL', detail: 'Missing heading', issues: ['Problem is missing'] }];
  assert.match(flightReport({ stages, sha: 'abc', url: '' }), /Problem is missing/);
  assert.equal(guidance([]).title, 'Check results are missing');
});

test('rerun commands are deduplicated, safe, and distinguish cancellation from failed jobs', () => {
  assert.equal(retryCommands([blocked, blocked]).length, 1);
  assert.deepEqual(retryCommands([{ ...blocked, state: 'PENDING' }, { ...blocked, url: 'https://github.com/x/evil`cmd`/actions/runs/1' }]), []);
  assert.deepEqual(retryCommands([{ ...blocked, state: 'CANCELLED' }]), ['gh run rerun 40294743172 --repo oleg-koval/vlag']);
});

test('PR-description and policy failures identify the edit and verification required', () => {
  const pr = guidance([{ name: 'PR sections', state: 'FAIL', issues: ['Problem is missing'] }]);
  assert.match(pr.actions.join(' '), /Edit the PR description.*new PR-description check/);
  const policy = guidance([{ name: 'Policy mapping', state: 'FAIL', issues: ['C7: no check'] }]);
  assert.match(policy.actions.join(' '), /configured policy directory.*charter doctor/);
});
