import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync, appendFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
const stable = /^v?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;
const parts = (value) => stable.exec(value)?.slice(1).map(Number);
const compare = (left, right) => left[0] - right[0] || left[1] - right[1] || left[2] - right[2];

export function nextVersion(tags, declared) {
  const current = parts(declared);
  if (!current) throw new Error('Cargo version must be stable semver');
  const latest = tags.map(parts).filter(Boolean).sort(compare).at(-1);
  if (!latest || compare(current, latest) > 0) return current.join('.');
  return [latest[0], latest[1], latest[2] + 1].join('.');
}
export function releaseFiles(cargo, action, version) {
  if (!/^version = "[^"]+"$/m.test(cargo) || !/^    default: v\d+\.\d+\.\d+$/m.test(action)) {
    throw new Error('Missing package version or action release default');
  }
  return {
    cargo: cargo.replace(/^version = "[^"]+"$/m, `version = "${version}"`),
    action: action.replace(/^    default: v\d+\.\d+\.\d+$/m, `    default: v${version}`),
  };
}
function prepare() {
  const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();
  const head = git('rev-parse', 'HEAD');
  const tags = git('tag', '--list', 'v*').split('\n').filter(Boolean);
  const existing = tags.filter((tag) => stable.test(tag)).find((tag) =>
    git('show', '-s', '--format=%B', `${tag}^{commit}`).includes(`Charter-Source: ${head}`));
  if (existing) {
    appendFileSync(process.env.GITHUB_OUTPUT, `tag=${existing}\n`);
    return;
  }
  const cargo = readFileSync('Cargo.toml', 'utf8');
  const version = nextVersion(tags, /^version = "([^"]+)"$/m.exec(cargo)?.[1]);
  const tag = `v${version}`;
  const files = releaseFiles(cargo, readFileSync('action.yml', 'utf8'), version);
  writeFileSync('Cargo.toml', files.cargo);
  writeFileSync('action.yml', files.action);
  execFileSync('cargo', ['update', '--offline', '--package', 'charter-cli'], { stdio: ['ignore', 'ignore', 'inherit'] });
  git('config', 'user.name', 'github-actions[bot]');
  git('config', 'user.email', '41898282+github-actions[bot]@users.noreply.github.com');
  git('add', 'Cargo.toml', 'Cargo.lock', 'action.yml');
  git('commit', '-m', `chore(release): align metadata for ${tag}\n\nCharter-Source: ${head}`);
  git('tag', '-a', tag, '-m', `Charter ${tag}; source ${head}`);
  git('push', 'origin', `refs/tags/${tag}`);
  appendFileSync(process.env.GITHUB_OUTPUT, `tag=${tag}\n`);
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, `Release ${tag} from tested commit ${head}.\n`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) prepare();
