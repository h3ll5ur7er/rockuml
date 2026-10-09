// Assembles the npm packages in dist/npm/core and dist/npm/angular, each with its license texts, ready for
// `npm publish dist/npm/<package>`. Needs web/rockuml.wasm (tools/build-wasm.sh), the built library
// (npm run build:library) and the collected licenses (tools/collect-licenses.sh), all of which CI makes anyway.
//   node tools/stage-npm-packages.mjs

import { access, cp, readFile, rm } from 'node:fs/promises';

const repository = new URL('../../', import.meta.url);
const output = new URL('../dist/npm/', import.meta.url);
const legal = new URL('legal/', repository);

const packages = {
  core: {
    from: new URL('web/', repository),
    // The demo page and the tests stay behind.
    files: ['package.json', 'README.md', 'rockuml.js', 'rockuml.d.ts', 'rockuml.wasm'],
    // The wasm module carries the fonts, Smetana and Rust crates, whose notices go with it.
    legal: ['LICENSE', 'COPYING', 'NOTICE.md', 'THIRD-PARTY.md', 'THIRD-PARTY-CRATES.md', 'crates'],
  },
  angular: {
    from: new URL('dist/rockuml-angular/', new URL('../', import.meta.url)),
    files: ['.'],
    legal: ['LICENSE', 'COPYING'],
  },
};

await rm(output, { recursive: true, force: true });
for (const [name, { from, files, legal: notices }] of Object.entries(packages)) {
  const target = new URL(`${name}/`, output);
  for (const file of files) {
    await copy(new URL(file, from), new URL(file, target));
  }
  for (const notice of notices) {
    await copy(new URL(notice, legal), new URL(notice, target));
  }
  const { name: packageName, version } = JSON.parse(
    await readFile(new URL('package.json', target), 'utf8'),
  );
  console.log(`${packageName}@${version} in dist/npm/${name}`);
}

async function copy(source, target) {
  try {
    await access(source);
  } catch {
    throw new Error(
      `${source.pathname} is missing: build it first (see the comment at the top of this script)`,
    );
  }
  await cp(source, target, { recursive: true });
}
