import { guidance, resultText, retryCommands } from './guidance.mjs';

export const cell = (value) => String(value).slice(0, 1500)
  .replace(/[&<>@|`\[\]\\]/g, (char) => `&#${char.codePointAt(0)};`)
  .replace(/[\r\n]/g, ' ');

const lamps = { PASS: '🟢', FAIL: '🔴', 'INFRA ERROR': '🔴', 'NOT RUN': '⚪', CANCELLED: '⚪', PENDING: '🟡' };
const names = { 'PR sections': 'PR description', 'Policy mapping': 'Policy configuration', 'Mapped code checks': 'Code checks' };
const row = (stage) => `| ${stage.url ? `[${cell(names[stage.name] || stage.name)}](${stage.url})` : cell(names[stage.name] || stage.name)} | ${lamps[stage.state]} ${resultText(stage)} |`;

export function flightReport({ scope, sha, url, stages, waived = null }) {
  const next = guidance(stages);
  const commands = retryCommands(stages, url);
  return [
    '## ✈ Charter check report', `### ${next.title}`, next.reason, '',
    ...(next.actions.length ? ['### What to do next', ...next.actions.map((action, index) => `${index + 1}. ${action}`), ''] : []),
    next.verify, '', '| Check | Result |', '| :-- | :-- |', ...stages.map(row), '',
    ...stages.flatMap((stage) => stage.state === 'FAIL' ? stage.issues.map((issue) => `- **${cell(stage.name)}:** ${cell(issue)}`) : []),
    ...(commands.length ? ['<details><summary>Rerun commands for the agent</summary>', '',
      'Run after fixing the cause. If the PR description changed, trigger a fresh PR-description check instead of rerunning its old event.',
      '', '```sh', ...commands, '```', '', '</details>', ''] : []),
    '<details><summary>Technical details and rule exceptions</summary>', '',
    `Commit: \`${cell(sha)}\` · ${cell(scope)} · [Open checks and logs](${url})`, '',
    ...stages.flatMap((stage) => [`- **${cell(stage.name)} · ${stage.state}:** ${cell(stage.detail)}`,
      ...stage.issues.map((issue) => `  - ${cell(issue)}`)]), '',
    `Waiver register · ${waived === null ? 'UNAVAILABLE: these runs did not report rule exceptions.' : waived.length}`,
    ...(waived || []).map((waiver) => `- **${cell(waiver.rule)}** · ${cell(waiver.reason)}`),
    ...(waived?.length === 0 ? ['No waivers reported by this run.'] : []),
    '', 'Waivers are rules without an automated check. They are not passes.',
    'These results are not merge clearance. Independent review is not verified here.', '', '</details>', '',
  ].join('\n');
}

export const annotation = (level, title, message) =>
  `::${level} title=${title}::${String(message).replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A')}`;
