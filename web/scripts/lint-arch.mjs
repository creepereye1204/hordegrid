// Mechanical architecture checks (docs/FRONTEND.md#모듈-경계, docs/design-docs/determinism.md#금지).
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const root = new URL('../..', import.meta.url).pathname;
const errors = [];
const walk = (dir, ext) =>
  readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    if (f === 'node_modules' || f === 'wasm-pkg' || f === 'dist') return [];
    return statSync(p).isDirectory() ? walk(p, ext) : p.endsWith(ext) ? [p] : [];
  });

// TS layer rules: module -> forbidden import prefixes
const FORBIDDEN = {
  'src/net/': ['../render', '../ui', '../input', '../match'],
  'src/render/': ['../net', '../ui', '../input', '../match'],
  'src/input/': ['../net', '../render', '../ui', '../match'],
  'src/core/': ['../net', '../render', '../ui', '../input', '../match'],
};
for (const file of walk(join(root, 'web/src'), '.ts')) {
  const rel = relative(join(root, 'web'), file);
  const rules = Object.entries(FORBIDDEN).find(([prefix]) => rel.startsWith(prefix));
  if (!rules) continue;
  for (const m of readFileSync(file, 'utf8').matchAll(/from\s+'([^']+)'/g)) {
    if (rules[1].some((bad) => m[1].startsWith(bad))) errors.push(`${rel}: forbidden import '${m[1]}'`);
  }
  if (/\binnerHTML\b/.test(readFileSync(file, 'utf8'))) errors.push(`${rel}: innerHTML is forbidden (docs/SECURITY.md)`);
}

// Rust sim: no float types/literals outside tests/examples
for (const file of walk(join(root, 'crates/sim/src'), '.rs')) {
  const src = readFileSync(file, 'utf8').split('#[cfg(test)]')[0];
  src.split('\n').forEach((line, i) => {
    const code = line.replace(/\/\/.*$/, '');
    if (/\bf(32|64)\b|\d+\.\d+/.test(code)) errors.push(`${relative(root, file)}:${i + 1}: float in sim`);
  });
}

if (errors.length) {
  console.error(errors.join('\n'));
  process.exit(1);
}
console.log('lint-arch ok');
