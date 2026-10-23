import { defineConfig } from 'vite';
import wasm from 'vite-plugin-wasm';
import topLevelAwait from 'vite-plugin-top-level-await';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '../..');

function wasmAutoRebuild() {
  let building = false;
  let queued = false;

  const runBuild = (server) => {
    if (building) {
      queued = true;
      return;
    }
    building = true;
    const start = Date.now();
    server.config.logger.info('\x1b[36m[wasm]\x1b[0m rebuilding wasm (dev mode)...', { timestamp: true });

    const proc = spawn(
      'wasm-pack',
      [
        'build',
        '--dev',
        '--target',
        'bundler',
        '--out-dir',
        'examples/wasm/pkg',
        rootDir,
        '--',
        '--features',
        'wasm',
      ],
      { stdio: 'inherit', cwd: rootDir }
    );

    proc.on('close', (code) => {
      building = false;
      const duration = ((Date.now() - start) / 1000).toFixed(2);
      if (code === 0) {
        server.config.logger.info(`\x1b[32m[wasm]\x1b[0m built in ${duration}s`, { timestamp: true });
        server.ws.send({ type: 'full-reload' });
      } else {
        server.config.logger.error(`\x1b[31m[wasm]\x1b[0m build failed with code ${code}`, {
          timestamp: true,
        });
      }
      if (queued) {
        queued = false;
        runBuild(server);
      }
    });
  };

  return {
    name: 'wasm-auto-rebuild',
    configureServer(server) {
      const srcDir = path.resolve(rootDir, 'src');
      const resourcesDir = path.resolve(rootDir, 'resources');
      const cargoToml = path.resolve(rootDir, 'Cargo.toml');

      server.watcher.add([srcDir, resourcesDir, cargoToml]);

      let timer = null;
      const handleChange = (file) => {
        if (!file.startsWith(srcDir) && !file.startsWith(resourcesDir) && file !== cargoToml) {
          return;
        }
        clearTimeout(timer);
        timer = setTimeout(() => runBuild(server), 150);
      };

      server.watcher.on('change', handleChange);
      server.watcher.on('add', handleChange);
      server.watcher.on('unlink', handleChange);
    },
  };
}

export default defineConfig({
  base: process.env.BASE_URL || '/',
  plugins: [wasmAutoRebuild(), wasm(), topLevelAwait()],
});
