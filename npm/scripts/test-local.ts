import { execSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { platform, arch } from 'node:process';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const repoRoot = path.resolve(root, '..');

const platformMap: Record<string, { target: string; triple: string; binName: string }> = {
  'darwin:arm64': { target: 'darwin-arm64', triple: 'aarch64-apple-darwin',     binName: 'regxmllib-rs' },
  'darwin:x64':   { target: 'darwin-x64',   triple: 'x86_64-apple-darwin',       binName: 'regxmllib-rs' },
  'linux:arm64':  { target: 'linux-arm64',  triple: 'aarch64-unknown-linux-gnu', binName: 'regxmllib-rs' },
  'linux:x64':    { target: 'linux-x64',    triple: 'x86_64-unknown-linux-gnu',  binName: 'regxmllib-rs' },
  'win32:x64':    { target: 'win32-x64',    triple: 'x86_64-pc-windows-msvc',    binName: 'regxmllib-rs.exe' },
  'win32:arm64':  { target: 'win32-arm64',  triple: 'aarch64-pc-windows-msvc',   binName: 'regxmllib-rs.exe' },
};

const key = `${platform}:${arch}`;
const info = platformMap[key];
if (!info) {
  console.error(`[test-local] Unsupported platform: ${key}`);
  process.exit(1);
}

const { target, triple, binName } = info;

// 1. Build
console.log(`\n[test-local] Building for ${triple}...`);
const build = spawnSync(
  'cargo',
  ['build', '-p', 'regxmllib-cli', '--bin', 'regxmllib-rs', '--release', '--target', triple],
  { cwd: repoRoot, stdio: 'inherit' }
);
if (build.status !== 0) process.exit(build.status ?? 1);

// 2. Stage
const binaryPath = path.join(repoRoot, 'target', triple, 'release', binName);
const destDir = path.join(root, `regxmllib-rs-${target}`, 'bin');
fs.mkdirSync(destDir, { recursive: true });
const destPath = path.join(destDir, binName);
fs.copyFileSync(binaryPath, destPath);
if (!binName.endsWith('.exe')) fs.chmodSync(destPath, 0o755);
console.log(`[test-local] Staged → ${destPath}`);

// 3. Pack
console.log('\n[test-local] Packing...');
const platformPkg = `regxmllib-rs-${target}`;

function packTo(pkgDir: string, outDir: string): string {
  const out = execSync(`pnpm pack --pack-destination ${outDir}`, { cwd: path.join(root, pkgDir) }).toString();
  // pnpm prints progress lines followed by the tarball path as the last non-empty line
  const tgz = out.split('\n').map((l) => l.trim()).filter((l) => l.endsWith('.tgz')).at(-1);
  if (!tgz) throw new Error(`pnpm pack produced no .tgz path:\n${out}`);
  return tgz;
}

const tmpDir = fs.mkdtempSync('/tmp/regxmllib-test-');
const platformTgz = packTo(platformPkg, tmpDir);
const launcherTgz = packTo('regxmllib-rs', tmpDir);
console.log(`[test-local] Packed → ${platformTgz}`);
console.log(`[test-local] Packed → ${launcherTgz}`);

// 4. Install into temp dir
console.log('\n[test-local] Installing into temp dir...');
const installDir = fs.mkdtempSync('/tmp/regxmllib-install-');
const install = spawnSync(
  'npm',
  ['install', platformTgz, launcherTgz],
  { cwd: installDir, stdio: 'inherit', shell: true }
);
if (install.status !== 0) process.exit(install.status ?? 1);

// 5. Smoke test
console.log('\n[test-local] Running smoke test...\n');
const bin = path.join(installDir, 'node_modules', '.bin', 'regxmllib-rs');
const smoke = spawnSync(bin, ['RegXMLDump', '--help'], { stdio: 'inherit' });

// Cleanup
fs.rmSync(tmpDir, { recursive: true, force: true });
fs.rmSync(installDir, { recursive: true, force: true });

if (smoke.status !== 0) {
  console.error('\n[test-local] Smoke test failed');
  process.exit(smoke.status ?? 1);
}
console.log('\n[test-local] OK');
