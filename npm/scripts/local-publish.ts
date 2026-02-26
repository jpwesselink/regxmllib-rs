import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { platform, arch } from 'node:process';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');
const repoRoot = path.resolve(root, '..');

interface PlatformInfo {
  target: string;
  triple: string;
  binName: string;
}

const platformMap: Record<string, PlatformInfo> = {
  'darwin:arm64': { target: 'darwin-arm64', triple: 'aarch64-apple-darwin',        binName: 'regxmllib-rs' },
  'darwin:x64':   { target: 'darwin-x64',   triple: 'x86_64-apple-darwin',          binName: 'regxmllib-rs' },
  'linux:arm64':  { target: 'linux-arm64',  triple: 'aarch64-unknown-linux-gnu',    binName: 'regxmllib-rs' },
  'linux:x64':    { target: 'linux-x64',    triple: 'x86_64-unknown-linux-gnu',     binName: 'regxmllib-rs' },
  'win32:x64':    { target: 'win32-x64',    triple: 'x86_64-pc-windows-msvc',       binName: 'regxmllib-rs.exe' },
  'win32:arm64':  { target: 'win32-arm64',  triple: 'aarch64-pc-windows-msvc',      binName: 'regxmllib-rs.exe' },
};

const key = `${platform}:${arch}`;
const info = platformMap[key];
if (!info) {
  console.error(`[local-publish] Unsupported platform: ${key}`);
  process.exit(1);
}

const doPublish = process.argv.includes('--publish');
const { target, triple, binName } = info;

// 1. Build
console.log(`\n[local-publish] Building regxmllib-rs for ${triple}...`);
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
console.log(`[local-publish] Staged → ${destPath}`);

// 3. Validate
const check = spawnSync('node', ['./scripts/check-packages.ts'], { cwd: root, stdio: 'inherit' });
if (check.status !== 0) process.exit(check.status ?? 1);

// 4. Pack or publish
const platformPkg = `./regxmllib-rs-${target}`;
const launcherPkg = './regxmllib-rs';

if (!doPublish) {
  console.log('\n[local-publish] Dry run — packing only (pass --publish to publish)\n');
  for (const pkg of [platformPkg, launcherPkg]) {
    const r = spawnSync('npm', ['pack', pkg], { cwd: root, stdio: 'inherit', shell: true });
    if (r.status !== 0) process.exit(r.status ?? 1);
  }
} else {
  console.log('\n[local-publish] Publishing to npmjs.com...\n');
  for (const pkg of [platformPkg, launcherPkg]) {
    const r = spawnSync('npm', ['publish', pkg, '--access', 'public'], { cwd: root, stdio: 'inherit', shell: true });
    if (r.status !== 0) process.exit(r.status ?? 1);
  }
}
