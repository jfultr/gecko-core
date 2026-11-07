// Generic library builder. Usage: node build.mjs <lib> <variant>
// Runs ./<lib>/build.mjs (exports `variants` and `build(variant, ctx)` returning {js, css})
// and writes dist/<lib>-<variant>.js and .css, safe to place inside <script>/<style>.
// Normally run through `python3 tools/articles/articles.py libs build <lib> <variant>`.
import fs from 'node:fs';
import path from 'node:path';

const root = import.meta.dirname;
const [lib, variant] = process.argv.slice(2);
if (!lib || !variant || !/^[a-z0-9-]+$/.test(lib) || lib.startsWith('_') || !fs.existsSync(path.join(root, lib, 'build.mjs'))) {
  console.error('usage: node build.mjs <lib> <variant>   (lib = a folder with build.mjs, see README.md)');
  process.exit(2);
}
const recipe = await import(`./${lib}/build.mjs`);
if (!recipe.variants.includes(variant)) {
  console.error(`${lib}: unknown variant '${variant}' (available: ${recipe.variants.join(', ')})`);
  process.exit(2);
}
const dist = path.join(root, 'dist');
const work = path.join(dist, `.work-${lib}`);
fs.mkdirSync(dist, { recursive: true });
const { js, css } = await recipe.build(variant, { root, work });
const safeJs = js.replaceAll('</script', '<\\/script');
const safeCss = (css || '').replaceAll('</style', '<\\/style');
fs.writeFileSync(path.join(dist, `${lib}-${variant}.js`), safeJs);
fs.writeFileSync(path.join(dist, `${lib}-${variant}.css`), safeCss);
fs.rmSync(work, { recursive: true, force: true });
console.log(`built ${lib} ${variant}: js ${safeJs.length} bytes, css ${safeCss.length} bytes`);
