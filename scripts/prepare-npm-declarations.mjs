import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)))
const packageName = process.argv[2]
const entryNames = process.argv.slice(3)

if (packageName === undefined || entryNames.length === 0) {
  console.error('usage: node scripts/prepare-npm-declarations.mjs <package-name> <entry...>')
  process.exit(1)
}

const packageDir = join(repoRoot, 'packages', packageName)
const distDir = join(packageDir, 'dist')
const sourceDeclarationDir = join(distDir, packageName, 'src')
const coreDeclaration = join(distDir, 'takanawa-js-core', 'src', 'index.d.ts')
const publicCoreDeclaration = join(distDir, 'core.d.ts')

copyDeclaration(coreDeclaration, publicCoreDeclaration)

for (const entryName of entryNames) {
  copyDeclaration(join(sourceDeclarationDir, `${entryName}.d.ts`), join(distDir, `${entryName}.d.ts`))
}

for (const supportName of ['definitions']) {
  const source = join(sourceDeclarationDir, `${supportName}.d.ts`)
  if (existsSync(source)) {
    copyDeclaration(source, join(distDir, `${supportName}.d.ts`))
  }
}

rmSync(join(distDir, packageName), { recursive: true, force: true })
rmSync(join(distDir, 'takanawa-js-core'), { recursive: true, force: true })

function copyDeclaration(source, destination) {
  const content = rewriteCoreImports(readFileSync(source, 'utf8'))
  mkdirSync(dirname(destination), { recursive: true })
  writeFileSync(destination, content)
}

function rewriteCoreImports(content) {
  return content.replaceAll("'takanawa-js-core'", "'./core'").replaceAll('"takanawa-js-core"', '"./core"')
}
