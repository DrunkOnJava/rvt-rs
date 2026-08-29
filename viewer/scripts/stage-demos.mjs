#!/usr/bin/env node
/**
 * Stage redistributable viewer demos into viewer/public/demos/.
 *
 * Reads docs/viewer-demos.json, copies/generates available binaries,
 * writes catalog.json + SVG thumbnails. Safe to run without the LFS
 * corpus — loadable RFAs are skipped with a warning; tier1 fixtures
 * copy from corpus/tier1/; the synthetic MVP fixture is generated when
 * gen-fixture is on PATH / in target/.
 *
 * Privacy: only redistributable files are staged. Nothing is uploaded.
 */

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const viewerRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(viewerRoot, '..');
const catalogSrc = path.join(repoRoot, 'docs', 'viewer-demos.json');
const publicDemos = path.join(viewerRoot, 'public', 'demos');
const thumbDir = path.join(publicDemos, 'thumbnails');

/** Known in-repo tier1 fixtures (Lane Three / M6-03). */
const TIER1_IDS = new Set(['architectural-2024', 'structural-2023', 'mep-2024']);

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function resolveExisting(candidates, base = repoRoot) {
  for (const candidate of candidates) {
    const resolved = path.isAbsolute(candidate)
      ? candidate
      : path.resolve(base, candidate);
    if (fs.existsSync(resolved)) return resolved;
  }
  return null;
}

function findGenFixture() {
  const binName = process.platform === 'win32' ? 'gen-fixture.exe' : 'gen-fixture';
  const candidates = [
    path.join(repoRoot, 'target', 'release', binName),
    path.join(repoRoot, 'target', 'debug', binName),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return candidate;
  }
  const pathCmd = process.platform === 'win32' ? 'where' : 'which';
  const onPath = spawnSync(pathCmd, ['gen-fixture'], {
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (onPath.status === 0) {
    const first = onPath.stdout.trim().split(/\r?\n/)[0];
    if (first) return first;
  }
  return null;
}

function writeThumbnail(filePath, label, accent) {
  const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="320" height="180" viewBox="0 0 320 180" role="img">
  <defs>
    <linearGradient id="g" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#11161d"/>
      <stop offset="100%" stop-color="${accent}"/>
    </linearGradient>
  </defs>
  <rect width="320" height="180" fill="url(#g)"/>
  <rect x="18" y="18" width="284" height="144" fill="none" stroke="#2a4a6f" stroke-width="2"/>
  <text x="32" y="70" fill="#d7dce3" font-family="ui-sans-serif, system-ui, sans-serif" font-size="18" font-weight="600">${escapeXml(label)}</text>
  <text x="32" y="100" fill="#8b95a3" font-family="ui-sans-serif, system-ui, sans-serif" font-size="12">rvt-rs demo</text>
</svg>
`;
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, svg);
}

function escapeXml(text) {
  return String(text)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function copyFile(src, dest) {
  ensureDir(path.dirname(dest));
  fs.copyFileSync(src, dest);
  return fs.statSync(dest).size;
}

function stageSyntheticMvp(destRel) {
  const dest = path.join(viewerRoot, 'public', destRel);
  if (fs.existsSync(dest) && fs.statSync(dest).size > 0) {
    return { ok: true, size: fs.statSync(dest).size, note: 'already present' };
  }
  const bin = findGenFixture();
  if (!bin) {
    return {
      ok: false,
      note: 'gen-fixture not found — build with cargo build --release --bin gen-fixture',
    };
  }
  ensureDir(path.dirname(dest));
  const result = spawnSync(
    bin,
    [
      'demo',
      '--classes',
      'Wall,Level,Project,Column,Door',
      '--element-count',
      '25',
      '--year',
      '2024',
      '--output',
      dest,
    ],
    { encoding: 'utf8' },
  );
  if (result.status !== 0) {
    return {
      ok: false,
      note: `gen-fixture failed: ${result.stderr || result.stdout || 'unknown error'}`,
    };
  }
  return { ok: true, size: fs.statSync(dest).size, note: `generated via ${bin}` };
}

function stageTier1(demo, destAbs) {
  const candidates = [
    ...(demo.source_candidates ?? []),
    demo.provenance,
    path.join('corpus', 'tier1', demo.id, `${demo.id}.rvt`),
  ].filter(Boolean);
  const src = resolveExisting(candidates);
  if (!src) {
    return { ok: false, note: `tier1 source missing for ${demo.id}` };
  }
  const size = copyFile(src, destAbs);
  // Also stage license sidecar when present (attribution / VW1-22).
  const licenseSrc = src.replace(/\.rvt$/i, '.license.json');
  if (fs.existsSync(licenseSrc)) {
    const licenseDest = destAbs.replace(/\.rvt$/i, '.license.json');
    copyFile(licenseSrc, licenseDest);
  }
  return {
    ok: true,
    size,
    note: `copied from ${path.relative(repoRoot, src)}`,
  };
}

function thumbAccent(demo) {
  if (demo.format === 'ifc') return '#2e684b';
  if (TIER1_IDS.has(demo.id)) return '#2a4a6f';
  if (demo.id === 'synthetic-mvp') return '#3c5f86';
  return '#1a3a5f';
}

function main() {
  if (!fs.existsSync(catalogSrc)) {
    console.error(`missing catalog: ${catalogSrc}`);
    process.exit(1);
  }

  ensureDir(publicDemos);
  ensureDir(thumbDir);

  const catalog = JSON.parse(fs.readFileSync(catalogSrc, 'utf8'));
  const staged = [];
  const warnings = [];

  for (const demo of catalog.demos) {
    const destRel = demo.file.replace(/^\.?\/?/, '');
    const destAbs = path.join(viewerRoot, 'public', destRel);
    const thumbRel = (demo.thumbnail || '').replace(/^\.?\/?/, '');
    let available = false;
    let sizeBytes = demo.size_bytes ?? null;
    let stageNote = 'not staged';

    if (TIER1_IDS.has(demo.id)) {
      const result = stageTier1(demo, destAbs);
      available = result.ok;
      sizeBytes = result.size ?? sizeBytes;
      stageNote = result.note;
      if (!result.ok) warnings.push(`${demo.id}: ${result.note}`);
    } else if (demo.id === 'synthetic-mvp') {
      const result = stageSyntheticMvp(destRel);
      available = result.ok;
      sizeBytes = result.size ?? sizeBytes;
      stageNote = result.note;
      if (!result.ok) warnings.push(`${demo.id}: ${result.note}`);
    } else if (demo.format === 'ifc') {
      const src = resolveExisting([
        path.join(repoRoot, 'tests', 'fixtures', path.basename(demo.file)),
        ...(demo.provenance ? [demo.provenance] : []),
      ]);
      if (src) {
        sizeBytes = copyFile(src, destAbs);
        available = true;
        stageNote = `copied from ${path.relative(repoRoot, src)}`;
      } else {
        warnings.push(`${demo.id}: IFC fixture missing`);
      }
    } else if (demo.loadable) {
      const candidates = [
        ...(demo.source_candidates ?? []),
        path.join('_corpus', 'examples', 'Autodesk', path.basename(demo.file).replace(/_/g, '')),
        `_corpus/examples/Autodesk/racbasicsamplefamily-${demo.revit_version}.rfa`,
        `../_corpus/examples/Autodesk/racbasicsamplefamily-${demo.revit_version}.rfa`,
      ];
      const src = resolveExisting(candidates);
      if (src) {
        sizeBytes = copyFile(src, destAbs);
        available = true;
        stageNote = `copied from ${path.relative(repoRoot, src)}`;
      } else {
        warnings.push(
          `${demo.id}: source not found (optional corpus) — gallery will mark unavailable`,
        );
      }
    }

    if (thumbRel) {
      const thumbAbs = path.join(viewerRoot, 'public', thumbRel);
      if (!fs.existsSync(thumbAbs)) {
        writeThumbnail(thumbAbs, demo.name, thumbAccent(demo));
      }
    }

    staged.push({
      ...demo,
      size_bytes: sizeBytes,
      available,
      stage_note: stageNote,
    });
  }

  const outCatalog = {
    ...catalog,
    demos: staged,
    staged_at: new Date().toISOString(),
  };
  fs.writeFileSync(
    path.join(publicDemos, 'catalog.json'),
    `${JSON.stringify(outCatalog, null, 2)}\n`,
  );

  const availableCount = staged.filter((d) => d.available).length;
  console.log(
    `staged ${availableCount}/${staged.length} demos → ${path.relative(repoRoot, publicDemos)}`,
  );
  for (const demo of staged) {
    console.log(`  - ${demo.id}: ${demo.available ? 'ok' : 'missing'} (${demo.stage_note})`);
  }
  for (const warning of warnings) {
    console.warn(`  warning: ${warning}`);
  }

  const hasLoadableRvt = staged.some(
    (d) => d.available && d.loadable && d.format === 'rvt',
  );
  if (!hasLoadableRvt) {
    console.warn(
      'warning: no loadable .rvt demo was staged (need corpus/tier1 or gen-fixture for Playwright coverage)',
    );
  }
}

main();
