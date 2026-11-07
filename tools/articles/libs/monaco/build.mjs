// Monaco recipe: bundles the read-only code viewer into one JS and one CSS file.
// Called by ../build.mjs (run via `articles.py libs build monaco <variant>`).
import { build as esbuild } from 'esbuild';
import fs from 'node:fs';
import path from 'node:path';

export const variants = ['full', 'min'];

// Languages included in the bundle (loaded lazily inside the bundle).
const languagesFull = new Set(['rust', 'shell', 'ini', 'python', 'javascript', 'typescript', 'json', 'yaml', 'markdown', 'cpp']);
const languagesMin = new Set(['rust', 'shell', 'ini', 'python', 'cpp']);
// Interactive features dropped from the small `min` variant.
const dropForMin = /contrib\/(?!tokenization|bracketMatching|unicodeHighlighter)|standalone\/browser\/(quickAccess|referenceSearch|inspectTokens|iPadShowKeyboard)|diffEditor|features\/find|gpuActions/;

export async function build(variant, { root, work }) {
  const esm = path.join(root, 'node_modules/monaco-editor/esm/vs');
  const editorDir = path.join(esm, 'editor');
  fs.mkdirSync(work, { recursive: true });

  // 1. The editor worker, bundled into one IIFE and embedded as text (started from a blob: URL).
  await esbuild({
    entryPoints: [path.join(editorDir, 'editor.worker.js')],
    bundle: true, minify: true, format: 'iife', legalComments: 'none', target: 'es2020',
    outfile: path.join(work, 'worker.txt'), logLevel: 'error',
  });

  // 2. Entry derived from the official editor.main.js without language services
  //    (css/html/json/ts) and without the language definitions we do not ship.
  const keepLangs = variant === 'min' ? languagesMin : languagesFull;
  const lines = fs.readFileSync(path.join(editorDir, 'editor.main.js'), 'utf8').split('\n').filter((l) => {
    if (/monaco-lsp-client|^export \{ index as lsp|__src_languages_features|languages\/features\/(css|html|json|typescript)\/register|^export \{ CancellationTokenSource/.test(l)) return false;
    const d = l.match(/languages\/definitions\/([^/]+)\/register/);
    if (d && !keepLangs.has(d[1])) return false;
    if (variant === 'min' && dropForMin.test(l)) return false;
    return true;
  });
  let src = lines.join('\n').replace(/(from|import) '(\.[^']+)'/g, (m, kw, p) => `${kw} '${path.resolve(editorDir, p)}'`);
  src += `
import * as api from '${path.join(editorDir, 'editor.api.js')}';
import WORKER_SRC from './worker.txt';
self.MonacoEnvironment = { getWorker: () => new Worker(URL.createObjectURL(new Blob([WORKER_SRC], { type: 'text/javascript' }))) };
window.monaco = api;
`;
  fs.writeFileSync(path.join(work, 'entry.js'), src);

  await esbuild({
    entryPoints: [path.join(work, 'entry.js')], bundle: true, minify: true, format: 'iife',
    outdir: path.join(work, 'out'), loader: { '.txt': 'text', '.ttf': 'dataurl' },
    legalComments: 'none', target: 'es2020', logLevel: 'error',
  });
  return {
    js: fs.readFileSync(path.join(work, 'out/entry.js'), 'utf8'),
    css: fs.readFileSync(path.join(work, 'out/entry.css'), 'utf8'),
  };
}
