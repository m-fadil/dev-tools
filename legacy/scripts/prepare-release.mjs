import { cp, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const destination = process.argv[2];
if (!destination) throw new Error('Usage: node scripts/prepare-release.mjs <empty-output-directory>');
const out = resolve(destination);
await mkdir(out);

const manifest = JSON.parse(await readFile(resolve(root, 'package.json'), 'utf8'));
// A Git-installed distribution must not run source preparation or require a compiler.
delete manifest.scripts;
delete manifest.devDependencies;
manifest.files = ['dist/src', 'README.md', 'docs'];
for (const entry of Object.values(manifest.bin)) {
    if (!(await stat(resolve(root, entry))).isFile()) throw new Error(`Missing compiled executable: ${entry}`);
}

const lock = JSON.parse(await readFile(resolve(root, 'package-lock.json'), 'utf8'));
lock.name = manifest.name;
lock.version = manifest.version;
lock.packages[''] = { ...lock.packages[''], name: manifest.name, version: manifest.version, bin: manifest.bin };
delete lock.packages[''].devDependencies;
for (const [name, entry] of Object.entries(lock.packages)) {
    if (entry.dev) delete lock.packages[name];
}

await cp(resolve(root, 'dist/src'), resolve(out, 'dist/src'), { recursive: true });
await cp(resolve(root, 'README.md'), resolve(out, 'README.md'));
await cp(resolve(root, 'docs'), resolve(out, 'docs'), { recursive: true });
await writeFile(resolve(out, 'package.json'), `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(resolve(out, 'package-lock.json'), `${JSON.stringify(lock, null, 2)}\n`);
console.log(`Prepared ${manifest.name}@${manifest.version} in ${out}`);
