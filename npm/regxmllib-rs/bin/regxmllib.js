#!/usr/bin/env node

const { spawnSync } = require('node:child_process');
const { platform, arch } = require('node:process');

const targetMap = {
  'darwin:x64': 'regxmllib-rs-darwin-x64',
  'darwin:arm64': 'regxmllib-rs-darwin-arm64',
  'linux:x64': 'regxmllib-rs-linux-x64',
  'linux:arm64': 'regxmllib-rs-linux-arm64',
  'win32:x64': 'regxmllib-rs-win32-x64',
  'win32:arm64': 'regxmllib-rs-win32-arm64'
};

const key = `${platform}:${arch}`;
const packageName = targetMap[key];
if (!packageName) {
  console.error(`[regxmllib-rs] Unsupported platform: ${key}`);
  process.exit(1);
}

const binaryName = platform === 'win32' ? 'regxmllib-rs.exe' : 'regxmllib-rs';

let binaryPath;
try {
  binaryPath = require.resolve(`${packageName}/bin/${binaryName}`);
} catch {
  console.error(`[regxmllib-rs] Missing platform binary package: ${packageName}`);
  console.error('[regxmllib-rs] Reinstall the package, or install a supported platform binary.');
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit'
});

if (result.error) {
  console.error(`[regxmllib-rs] Failed to execute binary: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);
