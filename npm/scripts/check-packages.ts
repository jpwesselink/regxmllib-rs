import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

const packages = [
  'regxmllib-rs',
  'regxmllib-rs-darwin-x64',
  'regxmllib-rs-darwin-arm64',
  'regxmllib-rs-linux-x64',
  'regxmllib-rs-linux-arm64',
  'regxmllib-rs-win32-x64',
  'regxmllib-rs-win32-arm64',
];

interface PackageManifest {
  name?: string;
  version?: string;
}

let failed = false;
for (const pkg of packages) {
  const manifestPath = path.join(root, pkg, 'package.json');
  if (!fs.existsSync(manifestPath)) {
    console.error(`[check-packages] Missing ${manifestPath}`);
    failed = true;
    continue;
  }
  const manifest: PackageManifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
  if (!manifest.name || !manifest.version) {
    console.error(`[check-packages] Invalid manifest: ${manifestPath}`);
    failed = true;
  }
}

if (failed) process.exit(1);
console.log('[check-packages] OK');
