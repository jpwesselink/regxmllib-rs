import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(path.dirname(new URL(import.meta.url).pathname), '..');

interface TargetInfo {
  packageDir: string;
  binName: string;
}

const targets: Record<string, TargetInfo> = {
  'darwin-x64':  { packageDir: 'regxmllib-rs-darwin-x64',  binName: 'regxmllib-rs' },
  'darwin-arm64':{ packageDir: 'regxmllib-rs-darwin-arm64', binName: 'regxmllib-rs' },
  'linux-x64':   { packageDir: 'regxmllib-rs-linux-x64',   binName: 'regxmllib-rs' },
  'linux-arm64': { packageDir: 'regxmllib-rs-linux-arm64',  binName: 'regxmllib-rs' },
  'win32-x64':   { packageDir: 'regxmllib-rs-win32-x64',   binName: 'regxmllib-rs.exe' },
  'win32-arm64': { packageDir: 'regxmllib-rs-win32-arm64',  binName: 'regxmllib-rs.exe' },
};

const args = process.argv.slice(2);
const options: Record<string, string> = Object.fromEntries(
  args
    .filter((arg) => arg.startsWith('--'))
    .map((arg) => {
      const [k, v] = arg.replace(/^--/, '').split('=');
      return [k, v ?? ''];
    })
);

const target = options.target;
const source = options.source;

if (!target || !targets[target]) {
  console.error('[stage-binary] Missing or invalid --target=<darwin-x64|darwin-arm64|linux-x64|linux-arm64|win32-x64|win32-arm64>');
  process.exit(1);
}

if (!source) {
  console.error('[stage-binary] Missing --source=/path/to/regxmllib-rs binary');
  process.exit(1);
}

const sourcePath = path.resolve(process.cwd(), source);
if (!fs.existsSync(sourcePath)) {
  console.error(`[stage-binary] Source binary not found: ${sourcePath}`);
  process.exit(1);
}

const { packageDir, binName } = targets[target];
const destPath = path.join(root, packageDir, 'bin', binName);
fs.mkdirSync(path.dirname(destPath), { recursive: true });
fs.copyFileSync(sourcePath, destPath);

if (!binName.endsWith('.exe')) {
  fs.chmodSync(destPath, 0o755);
}

console.log(`[stage-binary] Staged ${sourcePath} -> ${destPath}`);
