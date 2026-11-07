// Recipe skeleton. Copy to ../<lib>/build.mjs. Called by ../build.mjs.
// Return one JS string and one CSS string (css may be ''); the runner escapes </script and </style.
import { build as esbuild } from 'esbuild';
import fs from 'node:fs';
import path from 'node:path';

export const variants = ['full', 'min'];

export async function build(variant, { root, work }) {
  fs.mkdirSync(work, { recursive: true });
  await esbuild({
    entryPoints: [path.join(root, 'node_modules/PACKAGE/ENTRY.js')],
    bundle: true, minify: true, format: 'iife', legalComments: 'none', target: 'es2020',
    outdir: path.join(work, 'out'), logLevel: 'error',
  });
  return {
    js: fs.readFileSync(path.join(work, 'out/ENTRY.js'), 'utf8'),
    css: '',
  };
}
