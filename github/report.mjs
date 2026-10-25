export const cell = (value) => String(value).slice(0, 1500)
  .replace(/[&<>@|`\[\]\\]/g, (char) => `&#${char.codePointAt(0)};`)
  .replace(/[\r\n]/g, ' ');

const lamps = { PASS: '🟢', FAIL: '🔴', 'INFRA ERROR': '🔴', 'NOT RUN': '⚪', CANCELLED: '⚪', PENDING: '🟡' };
const row = (stage) => `| ${lamps[stage.state]} ${stage.state} | ${cell(stage.name)} | ${cell(stage.detail)} |`;

export function flightReport({ scope, sha, url, stages, waived = null }) {
  const clear = stages.length > 0 && stages.every((stage) => stage.state === 'PASS');
  return [
    '## ✈ Charter · FLIGHT CHECK',
    `### ${clear ? '🟢 PASS' : '🔴 HOLD'} · ${cell(scope)}`,
    `Commit: \`${cell(sha)}\` · [Open run and logs](${url})`,
    '', '| Signal | Station | Result |', '| :-- | :-- | :-- |', ...stages.map(row), '',
    '### 🔧 Required action',
    ...stages.flatMap((stage) => stage.issues.map((issue) => `- **${cell(stage.name)}:** ${cell(issue)}`)),
    ...(clear ? ['No failures in this scope. Other checks and human review still apply.'] : []),
    ...(!clear && stages.every((stage) => stage.issues.length === 0) ? ['Wait for the pending stations; no clearance has been issued.'] : []),
    '', `### 🟠 Waiver register · ${waived === null ? 'UNAVAILABLE' : waived.length}`,
    ...(waived || []).map((waiver) => `- **${cell(waiver.rule)}** · ${cell(waiver.reason)}`),
    ...(waived?.length === 0 ? ['No waivers reported by this run.'] : []),
    '', '> Scope-limited evidence, not merge clearance. Waived rules are not passes. Independent review is not verified here.', '',
  ].join('\n');
}

export const annotation = (level, title, message) =>
  `::${level} title=${title}::${String(message).replaceAll('%', '%25').replaceAll('\r', '%0D').replaceAll('\n', '%0A')}`;
