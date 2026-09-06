#!/usr/bin/env node
/**
 * Regenerate every derived table under the docs tree from the docs themselves.
 *
 * Nothing in markdown computes. This is what makes the derived state real:
 *
 * - a task's status is the count of its ticked acceptance criteria
 * - a roadmap item's status is its tasks' statuses, once it has a breakdown
 * - a roadmap item is Ready when every dependency is done, Blocked when one is not
 * - every table under the roadmap, explorations, ditched and tasks folders is a
 *   projection of the docs' metadata headers
 * - a breakdown folder is retired only once its roadmap item's header records the
 *   branch shipped (`Landed:` or `Merged:`) — reaching `done` never retires it
 *
 * A roadmap item is a folder, not a file. It lives at `docs/roadmap/NN-slug/` and
 * its `README.md` is the document this reads and rewrites. Anything else in that
 * folder — the contract (`openapi.json`), `wireframes/`, `sequence-diagrams.md`,
 * `test-script.md` — is permanent hand-written material: the engine never reads
 * it, never validates it, never rewrites it, and never deletes it, not even when
 * the item retires. (Explorations and ditched records stay single files.)
 *
 * What it never does: tick a checkbox, edit a task file, delete a task folder, or
 * touch hand-written prose. Those are judgement calls and belong to the skill.
 *
 * Paths come from `project.yml` at the repo root, so this file is the same in every
 * project that installs it. `msg init` writes that file.
 *
 *     node scripts/roadmap-sync.mjs            # write
 *     node scripts/roadmap-sync.mjs --check    # verify freshness, exit 1 on drift
 *
 * Vendored on purpose: the project owns its copy, so an upstream change cannot
 * silently alter how these tables render. Zero non-builtin imports, by rule.
 */

import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { parseArgs } from 'node:util';

export const DASH = '—';
export const STATUSES = ['not-started', 'in-progress', 'parked', 'done'];

const TITLE_RE = /^#\s+(\d+)\s+[—-]\s+(.*)$/;
const FIELD_RE = /\*\*(?<key>[A-Za-z ]+):\*\*\s*(?<value>[^·\n]*)/g;
const CHECKBOX_RE = /^\s*-\s+\[( |x)\]/i;
const TABLE_RE = /^\|.*\|$\n^\|[\s:|-]+\|$\n(?:^\|.*\|$\n)*/m;

const DEFAULT_STRUCTURE = {
  roadmap: 'docs/roadmap/',
  tasks: 'docs/tasks/',
  explorations: 'docs/explorations/',
  ditched: 'docs/ditched/',
};

const SYNC_COMMAND = 'make roadmap-sync';
const BREAKDOWN_SKILL = '/msg-roadmap-task-breakdown';

// A breakdown is retired only when its roadmap item's header records that the
// branch shipped — `Landed:` under GitButler, `Merged:` on plain git. Two
// spellings of one signal; the value is a free-text note (a date, a link).
// Order matters only for the message when a doc carries both: `Landed` wins.
const RETIRE_FIELDS = ['Landed', 'Merged'];

export class DocError extends Error {}

// --------------------------------------------------------------------------- io

/**
 * Read a file with newlines normalised, matching Python's universal-newline text
 * mode. Without this a CRLF checkout leaks a trailing `\r` into every parsed
 * title and field value.
 *
 * @param {string} path
 * @returns {string}
 */
export function readText(path) {
  return readFileSync(path, 'utf8').replace(/\r\n?/g, '\n');
}

/**
 * Split on newlines the way Python's `str.splitlines()` does: no trailing empty
 * element when the text ends in a newline.
 *
 * @param {string} text
 * @returns {string[]}
 */
export function splitLines(text) {
  if (text === '') return [];
  const lines = text.split('\n');
  if (lines[lines.length - 1] === '') lines.pop();
  return lines;
}

/**
 * Names in a directory matching a predicate, sorted by code unit — never
 * `localeCompare`, which reorders digits and punctuation. readdir order is
 * filesystem-dependent, so an unsorted listing is a silent nondeterminism bug.
 *
 * @param {string} dir
 * @param {(name: string) => boolean} predicate
 * @returns {string[]}
 */
function listSorted(dir, predicate) {
  if (!existsSync(dir) || !statSync(dir).isDirectory()) return [];
  return readdirSync(dir).filter(predicate).sort();
}

/** Matches the glob `[0-9]*.md`. @param {string} name */
const isNumberedDoc = (name) => /^[0-9].*\.md$/.test(name);

/** Matches the glob `[0-9]*-*`. @param {string} name */
const isNumberedFolder = (name) => /^[0-9]/.test(name) && name.indexOf('-', 1) !== -1;

// --------------------------------------------------------------------------- project.yml

/**
 * Nearest ancestor holding project.yml, else the repo root, else the start dir.
 *
 * @param {string} start
 * @returns {string}
 */
export function findRoot(start) {
  const chain = [];
  for (let dir = resolve(start); ; dir = dirname(dir)) {
    chain.push(dir);
    if (dirname(dir) === dir) break;
  }
  for (const dir of chain) if (existsSync(join(dir, 'project.yml'))) return dir;
  for (const dir of chain) if (existsSync(join(dir, '.git'))) return dir;
  return resolve(start);
}

/**
 * A deliberately tiny YAML subset: scalars, one level of nesting, flow lists.
 *
 * project.yml is a manifest, not a program. Supporting the whole language would
 * mean a dependency, and the schema `msg init` writes never needs one.
 *
 * Returns Maps rather than objects because integer-like object keys reorder in
 * JS, and the areas block is rendered back out in the order it was written.
 *
 * @param {string} text
 * @returns {Map<string, string | string[] | Map<string, string>>}
 */
export function parseSimpleYaml(text) {
  /** @type {Map<string, string | string[] | Map<string, string>>} */
  const root = new Map();
  /** @type {Map<string, string> | null} */
  let current = null;

  const unquote = (/** @type {string} */ v) => v.replace(/^['"]+|['"]+$/g, '');

  for (const raw of splitLines(text)) {
    const line = (raw.includes(' #') ? raw.slice(0, raw.indexOf(' #')) : raw).replace(/\s+$/, '');
    if (line.trim() === '' || line.trimStart().startsWith('#')) continue;

    const indented = line[0] === ' ' || line[0] === '\t';
    const trimmed = line.trim();
    // Only the first colon separates; a value may contain more.
    const colon = trimmed.indexOf(':');
    const key = (colon === -1 ? trimmed : trimmed.slice(0, colon)).trim();
    const value = (colon === -1 ? '' : trimmed.slice(colon + 1)).trim();

    if (indented && current !== null) {
      current.set(key, unquote(value));
      continue;
    }

    if (value === '') {
      current = new Map();
      root.set(key, current);
    } else if (value.startsWith('[') && value.endsWith(']')) {
      root.set(
        key,
        value
          .slice(1, -1)
          .split(',')
          .map((v) => unquote(v.trim()))
          .filter((v) => v !== ''),
      );
      current = null;
    } else {
      root.set(key, unquote(value));
      current = null;
    }
  }

  return root;
}

/**
 * @typedef {object} Config
 * @property {string} root
 * @property {string} roadmap
 * @property {string} tasks
 * @property {string} explorations
 * @property {string} ditched
 * @property {Array<{ block: string, key: string, value: string }>} manifestPaths
 * @property {(path: string) => string} rel
 */

/**
 * @param {string} [startDir]
 * @returns {Config}
 */
export function loadConfig(startDir) {
  const start = startDir ?? dirname(fileURLToPath(import.meta.url));
  const root = findRoot(start);

  /** @type {Map<string, unknown>} */
  let raw = new Map();
  const manifest = join(root, 'project.yml');
  if (existsSync(manifest)) raw = parseSimpleYaml(readText(manifest));

  const block = (/** @type {string} */ name) => {
    const value = raw.get(name);
    return value instanceof Map ? value : new Map();
  };
  const structure = block('structure');

  const folder = (/** @type {'roadmap'|'tasks'|'explorations'|'ditched'} */ name) =>
    join(root, String(structure.get(name) || DEFAULT_STRUCTURE[name]).replace(/\/+$/, ''));

  /** @type {Array<{ block: string, key: string, value: string }>} */
  const manifestPaths = [];
  if (existsSync(manifest)) {
    for (const name of ['structure', 'areas']) {
      for (const [key, value] of block(name)) manifestPaths.push({ block: name, key, value });
    }
    const requirementsFile = raw.get('requirementsFile');
    if (typeof requirementsFile === 'string' && requirementsFile.trim() !== '') {
      manifestPaths.push({ block: 'requirementsFile', key: '', value: requirementsFile });
    }
  }

  return {
    root,
    roadmap: folder('roadmap'),
    tasks: folder('tasks'),
    explorations: folder('explorations'),
    ditched: folder('ditched'),
    manifestPaths,
    rel(path) {
      const r = relative(root, path);
      return (r.startsWith('..') ? path : r).split('\\').join('/');
    },
  };
}

// --------------------------------------------------------------------------- parsing

/**
 * @typedef {object} Header
 * @property {number} number
 * @property {string} title
 * @property {Map<string, string>} fields
 * @property {string} text
 */

/**
 * Return the number, title, header fields and body of a numbered doc.
 *
 * `label` is what an error message calls the doc. It defaults to the file's own
 * name, which is right for a task or an exploration. A roadmap item's doc is
 * always named `README.md`, so a caller there passes the item folder's name
 * instead — otherwise every broken roadmap doc would just report `README.md`.
 *
 * @param {string} path
 * @param {string} [label]
 * @returns {Header}
 */
export function parseHeader(path, label) {
  const text = readText(path);
  const lines = splitLines(text);
  const name = label ?? basename(path);
  if (lines.length === 0) throw new DocError(`${name}: empty`);

  const titleMatch = TITLE_RE.exec(lines[0]);
  if (!titleMatch) throw new DocError(`${name}: first line is not \`# NN — Title\``);
  const number = Number(titleMatch[1]);
  const title = titleMatch[2].trim();

  const headerLine = lines.slice(1, 6).find((line) => line.startsWith('**'));
  if (headerLine === undefined) throw new DocError(`${name}: no \`**Key:** value\` metadata header`);

  /** @type {Map<string, string>} */
  const fields = new Map();
  // matchAll rather than a loop over .exec: a module-level /g regex leaks
  // lastIndex between calls.
  for (const m of headerLine.matchAll(FIELD_RE)) {
    const groups = m.groups;
    if (groups) fields.set(groups.key.trim(), groups.value.trim());
  }
  return { number, title, fields, text };
}

/**
 * `01, 02` -> ['01', '02']; an em dash or blank -> []. Kept as written.
 *
 * @param {string} raw
 * @returns {string[]}
 */
export function parseDeps(raw) {
  if (!raw || raw.trim() === DASH || raw.trim() === '-' || raw.trim() === '') return [];
  return raw
    .split(',')
    .map((part) => part.trim())
    .filter((part) => part !== '');
}

/** @param {number} n */
const pad = (n) => String(n).padStart(2, '0');

/** @param {string} s */
const isDigits = (s) => /^[0-9]+$/.test(s);

/**
 * @typedef {object} Task
 * @property {number} number
 * @property {string} title
 * @property {string} path
 * @property {string} scope
 * @property {string[]} deps
 * @property {number} ticked
 * @property {number} total
 */

/**
 * @typedef {object} RoadmapItem
 * @property {number} number
 * @property {string} title
 * @property {string} slug
 * @property {string} path
 * @property {string} estimate
 * @property {string} status
 * @property {string} retiredBy  the `Landed`/`Merged` header field that is set, or ''
 * @property {string[]} deps
 * @property {string} text
 * @property {Task[]} tasks
 */

/** @param {RoadmapItem | Task} x */
export const key = (x) => pad(x.number);

/** @param {RoadmapItem} item */
export const depNumbers = (item) => item.deps.filter(isDigits).map(Number);

/** The breakdown is retired once the branch has shipped. @param {RoadmapItem} item */
export const isRetired = (item) => Boolean(item.retiredBy);

// The link target for a roadmap item wherever a generated table names one. An
// item is a folder now, and `item.path` points at the `README.md` inside it. We
// link to `NN-slug/README.md`, not the bare folder, so a click opens the
// document instead of a directory listing. This helper is the single place that
// decides that, so the roadmap README cannot drift from it. (The tasks README
// links `NN-slug/` on purpose — that points at a task breakdown folder, a real
// directory of task files, not at a roadmap item.)
/** @param {RoadmapItem} item */
const itemLink = (item) => `[${key(item)}](${item.slug}/README.md)`;

/** @param {Task} task */
export function taskStatus(task) {
  if (task.total && task.ticked === task.total) return 'done';
  return task.ticked ? 'in-progress' : 'not-started';
}

/**
 * A roadmap item is the folder `docs/roadmap/NN-slug/`; its document is the
 * `README.md` inside it. We list the numbered folders, not files, and read each
 * folder's `README.md`. `item.slug` is the folder name; `item.path` is the path
 * to that `README.md`.
 *
 * Two things that used to be impossible are now just problems, not crashes:
 * a numbered folder with no `README.md`, and a leftover single file
 * `docs/roadmap/NN-slug.md` from before the folder layout. The second one names
 * the command that fixes it. Staying silent about either is how a repository
 * ends up half migrated.
 *
 * @param {Config} cfg
 * @param {string[]} problems
 * @returns {Map<number, RoadmapItem>}
 */
export function loadRoadmap(cfg, problems) {
  /** @type {Map<number, RoadmapItem>} */
  const items = new Map();
  const roadmapRel = cfg.rel(cfg.roadmap);

  for (const name of listSorted(cfg.roadmap, isNumberedFolder)) {
    const folder = join(cfg.roadmap, name);
    // `isNumberedFolder` also matches a name like `01-old.md`. Skip anything that
    // is not a directory here; the leftover-file check below reports those.
    if (!statSync(folder).isDirectory()) continue;

    const path = join(folder, 'README.md');
    if (!existsSync(path)) {
      problems.push(
        `${roadmapRel}/${name}: no README.md inside — that file is the item's document`,
      );
      continue;
    }

    const { number, title, fields, text } = parseHeader(path, name);
    const status = (fields.get('Status') ?? '').trim();
    if (!STATUSES.includes(status)) {
      throw new DocError(`${name}: status '${status}' not one of ${STATUSES.join(', ')}`);
    }
    const existing = items.get(number);
    if (existing) {
      throw new DocError(`${name}: number ${pad(number)} is already taken by ${existing.slug}/`);
    }
    items.set(number, {
      number,
      title,
      slug: name,
      path,
      estimate: (fields.get('Estimate') ?? '').trim(),
      status,
      retiredBy: RETIRE_FIELDS.find((f) => (fields.get(f) ?? '').trim() !== '') ?? '',
      deps: parseDeps(fields.get('Depends on') ?? ''),
      text,
      tasks: [],
    });
  }

  // A file `docs/roadmap/NN-slug.md` is the old layout. Do not try to parse it
  // and do not ignore it — tell the user the exact command that moves it.
  for (const name of listSorted(cfg.roadmap, isNumberedDoc)) {
    const slug = name.replace(/\.md$/, '');
    problems.push(
      `${roadmapRel}/${name}: a roadmap item is a folder now — run \`msg migrate-roadmap\` to move it to ${slug}/README.md`,
    );
  }

  return items;
}

/**
 * @param {Config} cfg
 * @param {Map<number, RoadmapItem>} items
 * @param {string[]} problems
 */
export function loadTasks(cfg, items, problems) {
  const tasksRel = cfg.rel(cfg.tasks);
  for (const folderName of listSorted(cfg.tasks, isNumberedFolder)) {
    const folder = join(cfg.tasks, folderName);
    if (!statSync(folder).isDirectory()) continue;

    const number = Number(folderName.slice(0, folderName.indexOf('-')));
    const item = items.get(number);
    if (item === undefined) {
      problems.push(`${tasksRel}/${folderName}: no roadmap doc ${pad(number)}`);
      continue;
    }
    if (folderName !== item.slug) {
      problems.push(
        `${tasksRel}/${folderName}: folder should be named ${item.slug} after its roadmap doc`,
      );
    }

    for (const name of listSorted(folder, isNumberedDoc)) {
      const path = join(folder, name);
      const { number: taskNumber, title, fields, text } = parseHeader(path);
      const marker = '## Acceptance criteria';
      const at = text.indexOf(marker);
      if (at === -1) {
        problems.push(`${tasksRel}/${folderName}/${name}: no acceptance criteria`);
        continue;
      }
      // indexOf + slice, never split: `split` has no maxsplit and would shatter a
      // doc that names the heading twice.
      const boxes = splitLines(text.slice(at + marker.length))
        .map((line) => CHECKBOX_RE.exec(line))
        .filter((m) => m !== null)
        .map((m) => m[1].toLowerCase());

      item.tasks.push({
        number: taskNumber,
        title,
        path,
        scope: (fields.get('Scope') ?? '').trim() || DASH,
        deps: parseDeps(fields.get('Depends on') ?? ''),
        ticked: boxes.filter((b) => b === 'x').length,
        total: boxes.length,
      });
    }
    item.tasks.sort((a, b) => a.number - b.number);
  }
}

// --------------------------------------------------------------------------- derivation

/**
 * A breakdown's checkboxes win over the roadmap doc's stored status.
 *
 * @param {Map<number, RoadmapItem>} items
 * @returns {string[]}
 */
export function deriveStatuses(items) {
  /** @type {string[]} */
  const changes = [];
  for (const item of items.values()) {
    if (item.tasks.length === 0) continue;
    if (item.status === 'parked') continue;
    const ticked = item.tasks.reduce((sum, t) => sum + t.ticked, 0);
    const total = item.tasks.reduce((sum, t) => sum + t.total, 0);
    const derived = total && ticked === total ? 'done' : ticked ? 'in-progress' : 'not-started';
    if (derived !== item.status) {
      changes.push(`roadmap ${key(item)} ${item.status} -> ${derived}`);
      item.status = derived;
      item.text = replaceField(item.text, 'Status', derived);
    }
  }
  return changes;
}

/**
 * Replace the first `**Key:** value` in the text, preserving the trailing space
 * that separates it from the next `·` field.
 *
 * @param {string} text
 * @param {string} keyName
 * @param {string} value
 * @returns {string}
 */
export function replaceField(text, keyName, value) {
  const escaped = keyName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  // No /g: first occurrence only, matching Python's count=1. A function
  // replacement keeps `$&` and `$1` inside `value` literal.
  return text.replace(
    new RegExp(`(\\*\\*${escaped}:\\*\\*\\s*)([^·\\n]*)`),
    (_match, prefix, old) => (old.endsWith(' ') ? `${prefix}${value} ` : `${prefix}${value}`),
  );
}

/**
 * @param {RoadmapItem} item
 * @param {Map<number, RoadmapItem>} items
 * @returns {string}
 */
export function sectionOf(item, items) {
  if (item.status === 'done') return 'Done';
  if (item.status === 'parked') return 'Parked';
  const blocked = depNumbers(item).some((n) => {
    const dep = items.get(n);
    return dep !== undefined && dep.status !== 'done';
  });
  return blocked ? 'Blocked' : 'Ready';
}

/**
 * @param {Config} cfg
 * @param {Map<number, RoadmapItem>} items
 * @param {string[]} problems
 */
export function validate(cfg, items, problems) {
  for (const item of items.values()) {
    for (const dep of item.deps) {
      if (!isDigits(dep)) {
        problems.push(`roadmap ${key(item)}: dependency '${dep}' is not a number`);
      } else if (!items.has(Number(dep))) {
        problems.push(`roadmap ${key(item)}: depends on ${dep}, which does not exist`);
      } else if (Number(dep) === item.number) {
        problems.push(`roadmap ${key(item)}: depends on itself`);
      }
    }
    if (item.status === 'done') {
      const openDeps = depNumbers(item).filter((d) => {
        const dep = items.get(d);
        return dep !== undefined && dep.status !== 'done';
      });
      if (openDeps.length) {
        problems.push(
          `roadmap ${key(item)}: done, but ${openDeps.map(pad).join(', ')} is not`,
        );
      }
    }
    // A present breakdown folder is fine while the work is in review — it only
    // becomes stale once the item's header says the branch shipped.
    if (isRetired(item)) {
      const label = item.retiredBy.toLowerCase();
      if (item.status !== 'done') {
        problems.push(
          `roadmap ${key(item)}: marked ${label}, but status is ${item.status}, not done`,
        );
      } else if (item.tasks.length) {
        problems.push(
          `roadmap ${key(item)}: ${label}, but ${cfg.rel(cfg.tasks)}/${item.slug}/ still exists — retire it`,
        );
      }
    }
    if (!isDigits(item.estimate)) {
      problems.push(`roadmap ${key(item)}: estimate '${item.estimate}' is not a number`);
    }
  }
}

/**
 * Every path named in project.yml must exist. That is the drift that happens.
 * Folded in here so `make roadmap-check` is the single gate.
 *
 * @param {Config} cfg
 * @param {string[]} problems
 */
export function validateManifest(cfg, problems) {
  for (const { block, key: name, value } of cfg.manifestPaths) {
    if (!existsSync(join(cfg.root, value))) {
      const label = name ? `${block}.${name}` : block;
      problems.push(`project.yml ${label} -> ${value} points at nothing`);
    }
  }
}

// --------------------------------------------------------------------------- rendering

/**
 * @param {string[]} headers
 * @param {string[][]} rows
 * @returns {string}
 */
export function renderTable(headers, rows) {
  if (rows.length === 0) return '_(none)_\n';
  const out = [`| ${headers.join(' | ')} |`, `|${headers.map(() => '---').join('|')}|`];
  for (const row of rows) out.push(`| ${row.join(' | ')} |`);
  return `${out.join('\n')}\n`;
}

/**
 * Heading row and separator only — a table that still parses as one.
 *
 * @param {string[]} headers
 * @returns {string}
 */
export function emptyTable(headers) {
  return `| ${headers.join(' | ')} |\n|${headers.map(() => '---').join('|')}|\n`;
}

/**
 * Estimate descending, then number ascending. A non-numeric estimate sorts as 0
 * rather than throwing — `validate` already reports it as a problem, and a bad
 * estimate should not stop the tables regenerating.
 *
 * @param {RoadmapItem[]} entries
 * @returns {RoadmapItem[]}
 */
export function sortQueue(entries) {
  const est = (/** @type {RoadmapItem} */ i) => (isDigits(i.estimate) ? Number(i.estimate) : 0);
  return [...entries].sort((a, b) => est(b) - est(a) || a.number - b.number);
}

/**
 * @param {Config} cfg
 * @param {Map<number, RoadmapItem>} items
 * @param {string} current
 * @returns {string}
 */
export function roadmapReadme(cfg, items, current) {
  const names = ['Ready', 'Blocked', 'Parked', 'Done'];
  /** @type {Record<string, RoadmapItem[]>} */
  const buckets = { Ready: [], Blocked: [], Parked: [], Done: [] };
  for (const item of items.values()) buckets[sectionOf(item, items)].push(item);

  const parts = names.map((name) => {
    const entries =
      name === 'Done'
        ? [...buckets[name]].sort((a, b) => a.number - b.number)
        : sortQueue(buckets[name]);
    const rows = entries.map((e) => [
      itemLink(e),
      e.title,
      e.estimate,
      e.deps.join(', ') || DASH,
      e.status,
    ]);
    return `## ${name}\n\n${renderTable(['#', 'Item', 'Est', 'Depends on', 'Status'], rows)}`;
  });

  const marker = current.indexOf('## Ready');
  if (marker === -1) {
    throw new DocError(
      `${cfg.rel(cfg.roadmap)}/README.md: no \`## Ready\` heading to regenerate from`,
    );
  }
  return current.slice(0, marker) + parts.join('\n');
}

/**
 * @param {string} text
 * @param {string} table
 * @param {string} where
 * @returns {string}
 */
export function replaceFirstTable(text, table, where) {
  const match = TABLE_RE.exec(text);
  if (!match) throw new DocError(`${where}: no table to regenerate`);
  return text.slice(0, match.index) + table + text.slice(match.index + match[0].length);
}

/**
 * @param {Config} cfg
 * @param {string} current
 * @returns {string}
 */
export function explorationsReadme(cfg, current) {
  const rows = listSorted(cfg.explorations, isNumberedDoc).map((name) => {
    const { number, title, fields } = parseHeader(join(cfg.explorations, name));
    return [
      `[${pad(number)}](${name})`,
      title,
      (fields.get('Estimate') ?? '').trim(),
      parseDeps(fields.get('Depends on') ?? '').join(', ') || DASH,
      (fields.get('Verdict') ?? '').trim(),
    ];
  });
  const est = (/** @type {string} */ v) => (isDigits(v) ? Number(v) : 0);
  rows.sort((a, b) => est(b[2]) - est(a[2]) || (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));

  const headers = ['#', 'Idea', 'Est', 'Depends on', 'Verdict'];
  // An empty table keeps its heading row: `_(none)_` would leave nothing for the
  // next run to find, and the run after that would fail with "no table to
  // regenerate". Only ever hit by a project with no explorations yet.
  const table = rows.length ? renderTable(headers, rows) : emptyTable(headers);
  return replaceFirstTable(current, table, `${cfg.rel(cfg.explorations)}/README.md`);
}

/**
 * @param {string} text
 * @param {string} heading
 * @returns {string}
 */
export function firstBullet(text, heading) {
  const at = text.indexOf(heading);
  if (at === -1) return '';
  for (const line of splitLines(text.slice(at + heading.length))) {
    if (line.startsWith('- ')) return line.slice(2).trim();
    if (line.startsWith('#')) break;
  }
  return '';
}

/**
 * @param {Config} cfg
 * @param {string} current
 * @returns {string}
 */
export function ditchedReadme(cfg, current) {
  const rows = listSorted(cfg.ditched, isNumberedDoc).map((name) => {
    const { number, title, fields, text } = parseHeader(join(cfg.ditched, name));
    return [
      `[${pad(number)}](${name})`,
      title,
      (fields.get('Ditched') ?? '').trim(),
      firstBullet(text, '## Why not'),
    ];
  });
  // Stable descending, matching Python's `sort(key=…, reverse=True)`.
  // `.sort().reverse()` would invert ties.
  rows.sort((a, b) => (a[2] < b[2] ? 1 : a[2] > b[2] ? -1 : 0));

  const headers = ['#', 'Idea', 'Ditched', 'Why not'];
  const table = rows.length ? renderTable(headers, rows) : emptyTable(headers);
  return replaceFirstTable(current, table, `${cfg.rel(cfg.ditched)}/README.md`);
}

/**
 * @param {Config} cfg
 * @param {RoadmapItem} item
 * @param {string} current
 * @returns {string}
 */
export function folderReadme(cfg, item, current) {
  const rows = item.tasks.map((t) => [
    `[${key(t)}](${basename(t.path)})`,
    t.title,
    t.scope,
    t.deps.join(', ') || DASH,
    `${t.ticked}/${t.total}`,
    taskStatus(t),
  ]);
  const table = renderTable(['#', 'Task', 'Scope', 'Depends on', 'Criteria', 'Status'], rows);
  return replaceFirstTable(current, table, `${cfg.rel(cfg.tasks)}/${item.slug}/README.md`);
}

/**
 * [1,2,3,10,15,16,17] -> '01–03, 10 and 15–17' — the tasks README's prose line.
 *
 * A run of two stays two numbers; a range of one is not a range.
 *
 * @param {number[]} numbers
 * @returns {string}
 */
export function compressNumbers(numbers) {
  if (numbers.length === 0) return '';
  /** @type {number[][]} */
  const groups = [[numbers[0]]];
  for (const n of numbers.slice(1)) {
    const last = groups[groups.length - 1];
    if (n === last[last.length - 1] + 1) last.push(n);
    else groups.push([n]);
  }
  /** @type {string[]} */
  const parts = [];
  for (const g of groups) {
    if (g.length > 2) parts.push(`${pad(g[0])}–${pad(g[g.length - 1])}`);
    else for (const n of g) parts.push(pad(n));
  }
  if (parts.length === 1) return parts[0];
  return `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]}`;
}

/** @returns {string} */
export function noBreakdownLine() {
  return `_No breakdown is open. Create one with \`${BREAKDOWN_SKILL} NN\`._\n`;
}

/**
 * @param {Config} cfg
 * @param {Map<number, RoadmapItem>} items
 * @param {string} current
 * @returns {string}
 */
export function tasksReadme(cfg, items, current) {
  const byNumber = [...items.values()].sort((a, b) => a.number - b.number);
  const openItems = byNumber.filter((i) => i.tasks.length > 0);
  const rows = openItems.map((i) => [
    `[${key(i)}](${i.slug}/)`,
    i.title,
    String(i.tasks.length),
    `${i.tasks.filter((t) => taskStatus(t) === 'done').length}/${i.tasks.length}`,
    i.status,
  ]);

  // An em dash for "none", the same convention the tables use. Without it an
  // empty list renders `Items  are \`done\`` — a double space in every freshly
  // scaffolded project, and the reason the seeded README would fail its own
  // first `make roadmap-check`.
  const done =
    compressNumbers(byNumber.filter((i) => i.status === 'done').map((i) => i.number)) || DASH;
  // No /g: first occurrence only. Function replacement keeps `$` in `done` literal.
  const text = current.replace(
    /Items [^\n]*(?:\n(?!\n)[^\n]*)* are `done`/,
    () => `Items ${done} are \`done\``,
  );

  const noneOpen = noBreakdownLine();
  if (rows.length) {
    const table = renderTable(['#', 'Roadmap item', 'Tasks', 'Progress', 'Status'], rows);
    if (text.includes(noneOpen.trim())) {
      return text.replace(noneOpen.trim(), () => table.replace(/\n+$/, ''));
    }
    return replaceFirstTable(text, table, `${cfg.rel(cfg.tasks)}/README.md`);
  }
  const match = TABLE_RE.exec(text);
  if (!match) return text;
  return text.slice(0, match.index) + noneOpen + text.slice(match.index + match[0].length);
}

// --------------------------------------------------------------------------- driver

/**
 * @typedef {object} RunResult
 * @property {number} code
 * @property {string[]} out
 * @property {string[]} err
 */

/**
 * @param {Config} cfg
 * @param {boolean} check
 * @returns {RunResult}
 */
export function run(cfg, check) {
  /** @type {string[]} */
  const problems = [];
  /** @type {string[]} */
  const out = [];
  /** @type {string[]} */
  const err = [];
  /** @type {Map<string, string>} */
  const writes = new Map();
  /** @type {string[]} */
  let statusChanges = [];
  /** @type {Map<number, RoadmapItem>} */
  let items = new Map();

  try {
    if (!existsSync(cfg.roadmap) || !statSync(cfg.roadmap).isDirectory()) {
      err.push(`error: ${cfg.rel(cfg.roadmap)}/ does not exist — run \`npx @lucas-gomide/msg-cli init\` first`);
      return { code: 2, out, err };
    }

    items = loadRoadmap(cfg, problems);
    loadTasks(cfg, items, problems);
    statusChanges = deriveStatuses(items);
    validate(cfg, items, problems);
    validateManifest(cfg, problems);

    for (const item of items.values()) {
      if (item.text !== readText(item.path)) writes.set(item.path, item.text);
    }

    const readme = join(cfg.roadmap, 'README.md');
    if (!existsSync(readme)) {
      throw new DocError(`${cfg.rel(readme)}: missing — it is the table this regenerates into`);
    }
    writes.set(readme, roadmapReadme(cfg, items, readText(readme)));

    /** @type {Array<[string, (cfg: Config, current: string) => string]>} */
    const folders = [
      [join(cfg.explorations, 'README.md'), explorationsReadme],
      [join(cfg.ditched, 'README.md'), ditchedReadme],
    ];
    for (const [path, builder] of folders) {
      if (existsSync(path)) writes.set(path, builder(cfg, readText(path)));
    }

    const tasksIndex = join(cfg.tasks, 'README.md');
    if (existsSync(tasksIndex)) {
      writes.set(tasksIndex, tasksReadme(cfg, items, readText(tasksIndex)));
    }
    for (const item of items.values()) {
      const path = join(cfg.tasks, item.slug, 'README.md');
      if (item.tasks.length && existsSync(path)) {
        writes.set(path, folderReadme(cfg, item, readText(path)));
      }
    }
  } catch (error) {
    if (!(error instanceof DocError)) throw error;
    err.push(`error: ${error.message}`);
    return { code: 2, out, err };
  }

  /** @type {Array<[string, string]>} */
  const stale = [...writes].filter(([path, content]) => readText(path) !== content);

  for (const line of statusChanges) out.push(`  status  ${line}`);
  for (const line of problems) out.push(`  problem ${line}`);

  if (check) {
    for (const [path] of stale) out.push(`  stale   ${cfg.rel(path)}`);
    if (stale.length || problems.length) {
      err.push(
        `\n${stale.length} file(s) stale, ${problems.length} problem(s). Run \`${SYNC_COMMAND}\`.`,
      );
      return { code: 1, out, err };
    }
    out.push('  roadmap tables are up to date');
    return { code: 0, out, err };
  }

  for (const [path, content] of stale) {
    writeFileSync(path, content, 'utf8');
    out.push(`  wrote   ${cfg.rel(path)}`);
  }
  if (stale.length === 0 && statusChanges.length === 0) {
    out.push('  nothing to do — every table already matches the docs');
  }

  const retire = [...items.values()]
    .filter((i) => isRetired(i) && i.status === 'done' && i.tasks.length)
    .sort((a, b) => a.number - b.number);
  for (const item of retire) {
    out.push(
      `  retire  ${cfg.rel(cfg.tasks)}/${item.slug}/ — ${key(item)} ${item.retiredBy.toLowerCase()}`,
    );
  }

  return { code: problems.length ? 1 : 0, out, err };
}

/**
 * @param {string[]} argv
 * @param {string} [startDir]
 * @returns {number}
 */
export function main(argv, startDir) {
  const { values } = parseArgs({
    args: argv,
    options: { check: { type: 'boolean', default: false } },
    allowPositionals: false,
  });
  const result = run(loadConfig(startDir), values.check === true);
  for (const line of result.out) process.stdout.write(`${line}\n`);
  for (const line of result.err) process.stderr.write(`${line}\n`);
  return result.code;
}

// Run only as a script, never on import — a top-level side effect would make
// every pure function above untestable.
//
// The filename check is not redundant. The CLI imports `parseSimpleYaml` from
// this file so the two can never disagree about a manifest, and its bundler
// inlines this module into its own entry point. There `import.meta.url` and
// `argv[1]` are the same file, so the URL comparison alone is true and the
// engine would hijack every CLI invocation. Only a file still named
// roadmap-sync.mjs — what `msg init` vendors — is allowed to self-execute.
const entry = process.argv[1];
const isVendoredScript = import.meta.url.endsWith('/roadmap-sync.mjs');
if (isVendoredScript && entry !== undefined && import.meta.url === pathToFileURL(entry).href) {
  process.exitCode = main(process.argv.slice(2));
}
