import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { basename, isAbsolute, join, relative, resolve, sep } from 'node:path'
import { spawnSync } from 'node:child_process'
import { parseArgs } from 'node:util'
import { fileURLToPath, pathToFileURL } from 'node:url'

const PROJECT_ROOT = fileURLToPath(new URL('..', import.meta.url))
const DIST_ROOT = join(PROJECT_ROOT, 'dist')
const DSH_ENTRY = join('node_modules', '@deepseek-ai', 'dsh', 'lib', 'bin.js')
const INSTALL_SCRIPT_POLICY = {
  '@google/genai': false,
  esbuild: true,
  koffi: true,
  'node-addon-require-builtin': false,
  'node-pty': true,
  protobufjs: false,
}

function usage() {
  return `Usage: pnpm package -- --dsh-root <checkout> --node-root <portable-node-directory> [options]

Options:
  --out <path>          Output directory under dist/ (default: dist/bundle)
  --skip-pack           Reuse tarballs already present under dist/packs
  --skip-dsh-build      Reuse existing dsh build artifacts
  --skip-native-build   Reuse the existing release dsh-deck executable
  --help                Show this help

The Node directory must be an unpacked official-style distribution containing
node.exe and node_modules/npm on Windows, or bin/node and lib/node_modules/npm
on Unix.`
}

function fail(message) {
  throw new Error(message)
}

function requireDirectory(path, label) {
  if (!existsSync(path)) fail(`${label} does not exist: ${path}`)
}

function assertSafeOutput(path) {
  const fromDist = relative(DIST_ROOT, path)
  if (fromDist === '' || fromDist.startsWith('..') || isAbsolute(fromDist)) {
    fail(`--out must name a child directory of ${DIST_ROOT}`)
  }
}

function shellArgument(value) {
  if (value.includes('"')) fail(`cannot pass an argument containing a double quote through cmd.exe: ${value}`)
  return /\s/.test(value) ? `"${value}"` : value
}

function run(command, args, options = {}) {
  const rendered = [command, ...args].join(' ')
  console.log(`> ${rendered}`)
  // PATH-resolved commands such as pnpm are .cmd shims on Windows, which Node
  // refuses to spawn directly, so those run as an explicitly quoted cmd.exe
  // command line instead.
  const shell = process.platform === 'win32' && !isAbsolute(command)
  const spawnOptions = {
    cwd: options.cwd,
    env: process.env,
    encoding: 'utf8',
    shell,
    stdio: options.capture ? ['ignore', 'pipe', 'inherit'] : 'inherit',
  }
  const result = shell
    ? spawnSync([command, ...args.map(shellArgument)].join(' '), spawnOptions)
    : spawnSync(command, args, spawnOptions)
  if (result.error) throw new Error(`failed to run ${rendered}`, { cause: result.error })
  if (result.status !== 0) fail(`${rendered} exited with status ${String(result.status)}`)
  return options.capture ? result.stdout.trim() : ''
}

function nodeDistribution(root) {
  const windows = process.platform === 'win32'
  const executable = windows ? join(root, 'node.exe') : join(root, 'bin', 'node')
  const npmCli = windows
    ? join(root, 'node_modules', 'npm', 'bin', 'npm-cli.js')
    : join(root, 'lib', 'node_modules', 'npm', 'bin', 'npm-cli.js')
  if (!existsSync(executable)) fail(`Node executable is missing: ${executable}`)
  if (!existsSync(npmCli)) fail(`npm CLI is missing from the Node distribution: ${npmCli}`)
  return { executable, npmCli }
}

function copyNodeRuntime(root, destination, node) {
  mkdirSync(destination, { recursive: true })
  if (process.platform === 'win32') {
    cpSync(node.executable, join(destination, 'node.exe'))
  } else {
    const bin = join(destination, 'bin')
    mkdirSync(bin)
    cpSync(node.executable, join(bin, 'node'))
  }
  for (const filename of ['LICENSE', 'README.md', 'CHANGELOG.md']) {
    const source = join(root, filename)
    if (existsSync(source)) cpSync(source, join(destination, filename))
  }
}

function tarballs(directory) {
  return readdirSync(directory, { withFileTypes: true })
    .filter(entry => entry.isFile() && entry.name.endsWith('.tgz'))
    .map(entry => join(directory, entry.name))
    .sort()
}

function tarballManifest(tarball) {
  const json = run('tar', ['-xOf', tarball, 'package/package.json'], { capture: true })
  const manifest = JSON.parse(json)
  if (typeof manifest.name !== 'string' || typeof manifest.version !== 'string') {
    fail(`${tarball} contains an invalid package manifest`)
  }
  return manifest
}

function packageDependencies(files) {
  const packages = new Map()
  for (const file of files) {
    const manifest = tarballManifest(file)
    if (packages.has(manifest.name)) fail(`duplicate package ${manifest.name}`)
    packages.set(manifest.name, { file, manifest })
  }

  const dependencies = {}
  const pending = ['@deepseek-ai/dsh']
  const visited = new Set()
  while (pending.length > 0) {
    const name = pending.pop()
    if (visited.has(name)) continue
    const local = packages.get(name)
    if (local === undefined) fail(`packed dsh dependency is missing: ${name}`)
    visited.add(name)
    dependencies[name] = pathToFileURL(local.file).href
    for (const section of ['dependencies', 'optionalDependencies', 'peerDependencies']) {
      const values = local.manifest[section]
      if (values === null || typeof values !== 'object' || Array.isArray(values)) continue
      for (const dependency of Object.keys(values)) {
        if (packages.has(dependency) && !visited.has(dependency)) pending.push(dependency)
      }
    }
  }
  console.log(`dsh runtime closure: ${String(visited.size)} of ${String(packages.size)} packed packages`)
  return { allowScripts: INSTALL_SCRIPT_POLICY, dependencies }
}

// The installed runtime tree ships development artifacts the runtime never
// loads. Vendor packages are trimmed to what dsh's own packages ship: compiled
// JS plus type declarations. First-party packages are left untouched, and
// declarations stay everywhere because runtime-composed plugins are written
// against these APIs.
const FIRST_PARTY_SCOPE = `${sep}node_modules${sep}@deepseek-ai${sep}`

function isDeclarationFile(name) {
  return /\.d\.[cm]?ts$/.test(name)
}

function isVendorOnlyArtifact(name) {
  if (name.endsWith('.map')) return true
  return /\.[cm]?ts$/.test(name) && !isDeclarationFile(name)
}

function pruneRuntimeTree(root) {
  const nativeTarget = `${process.platform}-${process.arch}`
  let bytes = 0
  let entries = 0
  const remove = (path, size) => {
    rmSync(path, { recursive: true, force: true })
    bytes += size
    entries += 1
  }
  const directorySize = path =>
    readdirSync(path, { recursive: true, withFileTypes: true })
      .filter(entry => entry.isFile())
      .reduce((sum, entry) => sum + statSync(join(entry.parentPath, entry.name)).size, 0)

  const walk = directory => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name)
      if (entry.isDirectory()) {
        if (basename(directory) === 'prebuilds' && entry.name !== nativeTarget) {
          remove(path, directorySize(path))
        } else {
          walk(path)
        }
        continue
      }
      if (!entry.isFile()) continue
      const firstParty = path.includes(FIRST_PARTY_SCOPE)
      if (entry.name.endsWith('.pdb') || (!firstParty && isVendorOnlyArtifact(entry.name))) {
        remove(path, statSync(path).size)
      }
    }
  }
  walk(root)
  console.log(
    `pruned ${(bytes / 1024 / 1024).toFixed(1)} MB of development artifacts (${String(entries)} entries)`,
  )
}

function nativeExecutable() {
  return join(
    PROJECT_ROOT,
    'native',
    'target',
    'release',
    process.platform === 'win32' ? 'dsh-deck.exe' : 'dsh-deck',
  )
}

function main() {
  const argv = process.argv.slice(2)
  if (argv[0] === '--') argv.shift()
  const { values } = parseArgs({
    args: argv,
    options: {
      'dsh-root': { type: 'string' },
      'node-root': { type: 'string' },
      out: { type: 'string' },
      'skip-pack': { type: 'boolean', default: false },
      'skip-dsh-build': { type: 'boolean', default: false },
      'skip-native-build': { type: 'boolean', default: false },
      help: { type: 'boolean', short: 'h', default: false },
    },
    allowPositionals: false,
  })

  if (values.help) {
    console.log(usage())
    return
  }
  if (values['dsh-root'] === undefined || values['node-root'] === undefined) fail(usage())

  const dshRoot = resolve(values['dsh-root'])
  const nodeRoot = resolve(values['node-root'])
  const output = resolve(PROJECT_ROOT, values.out ?? join('dist', 'bundle'))
  assertSafeOutput(output)
  requireDirectory(dshRoot, 'dsh checkout')
  requireDirectory(nodeRoot, 'Node distribution')
  const node = nodeDistribution(nodeRoot)

  if (!values['skip-dsh-build']) run('pnpm', ['run', 'build'], { cwd: dshRoot })
  if (!values['skip-native-build']) run('cargo', ['build', '--release', '--manifest-path', 'native/Cargo.toml'], { cwd: PROJECT_ROOT })

  mkdirSync(DIST_ROOT, { recursive: true })
  const staging = mkdtempSync(join(DIST_ROOT, '.bundle-'))
  try {
    const packs = join(DIST_ROOT, 'packs')
    const vendorPacks = join(packs, 'vendor')
    const dshPacks = join(packs, 'dsh')
    if (!values['skip-pack']) {
      run('pnpm', ['run', 'release:pack', '--family', 'vendor', '--out', vendorPacks], { cwd: dshRoot })
      run('pnpm', ['run', 'release:pack', '--family', 'dsh', '--out', dshPacks], { cwd: dshRoot })
    }
    requireDirectory(vendorPacks, 'vendor pack directory')
    requireDirectory(dshPacks, 'dsh pack directory')

    const files = [...tarballs(vendorPacks), ...tarballs(dshPacks)]
    if (files.length === 0) fail('dsh release packing produced no tarballs')

    const installRoot = join(staging, 'install')
    mkdirSync(installRoot)
    const runtimePackages = packageDependencies(files)
    writeFileSync(
      join(installRoot, 'package.json'),
      `${JSON.stringify(
        {
          name: 'dsh-deck-runtime',
          private: true,
          dependencies: runtimePackages.dependencies,
          allowScripts: runtimePackages.allowScripts,
        },
        undefined,
        2,
      )}\n`,
    )
    run(
      node.executable,
      [node.npmCli, 'install', '--ignore-scripts', '--no-audit', '--no-fund', '--package-lock=false'],
      { cwd: installRoot },
    )
    run(
      node.executable,
      [node.npmCli, 'rebuild', 'esbuild', 'koffi', 'node-pty'],
      { cwd: installRoot },
    )
    const subprocessRoot = join(
      installRoot,
      'node_modules',
      '@deepseek-ai',
      'dsh-subprocess-local',
    )
    run(node.executable, [join(subprocessRoot, 'scripts', 'ensure-spawn-helper.mjs')], {
      cwd: subprocessRoot,
    })
    pruneRuntimeTree(join(installRoot, 'node_modules'))

    const installedEntry = join(installRoot, DSH_ENTRY)
    if (!existsSync(installedEntry)) fail(`installed dsh entry is missing: ${installedEntry}`)
    writeFileSync(
      join(installRoot, 'package.json'),
      `${JSON.stringify({ name: 'dsh-deck-runtime', private: true }, undefined, 2)}\n`,
    )

    const bundle = join(staging, 'bundle')
    const runtime = join(bundle, 'runtime')
    mkdirSync(runtime, { recursive: true })
    copyNodeRuntime(nodeRoot, join(runtime, 'node'), node)
    cpSync(installRoot, join(runtime, 'dsh'), { recursive: true })

    const executable = nativeExecutable()
    if (!existsSync(executable)) fail(`native release executable is missing: ${executable}`)
    cpSync(executable, join(bundle, basename(executable)))
    cpSync(join(PROJECT_ROOT, 'LICENSE'), join(bundle, 'LICENSE'))
    cpSync(join(PROJECT_ROOT, 'THIRD_PARTY_NOTICES.md'), join(bundle, 'THIRD_PARTY_NOTICES.md'))

    const dshManifest = JSON.parse(
      readFileSync(join(runtime, 'dsh', 'node_modules', '@deepseek-ai', 'dsh', 'package.json'), 'utf8'),
    )
    writeFileSync(
      join(bundle, 'BUNDLE-MANIFEST.json'),
      `${JSON.stringify(
        {
          application: 'dsh-deck',
          platform: process.platform,
          architecture: process.arch,
          node: run(node.executable, ['--version'], { capture: true }),
          dsh: dshManifest.version,
        },
        undefined,
        2,
      )}\n`,
    )

    rmSync(output, { recursive: true, force: true })
    try {
      cpSync(bundle, output, { recursive: true })
    } catch (error) {
      rmSync(output, { recursive: true, force: true })
      throw error
    }
    console.log(`DSH Deck bundle: ${output}`)
  } finally {
    rmSync(staging, { recursive: true, force: true })
  }
}

try {
  main()
} catch (error) {
  console.error(error instanceof Error ? error.message : error)
  if (error instanceof Error && error.cause !== undefined) console.error(error.cause)
  process.exitCode = 1
}
