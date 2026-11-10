// Builds the engine as part of the app: `cargo chores wasm` before Vite
// starts (the quicker wasm-dev profile for `vite`, release for `vite
// build`), and again in development whenever a Rust file under engine/
// changes, followed by a full page reload.
//
// ENGINE_PREBUILT=1 skips the start-up build and uses the engine already
// in src/engine-room/wasm-build/, as the root scripts and CI do after
// building it once.

import { spawn, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import type { Plugin } from 'vite';

const REPOSITORY = fileURLToPath(new URL('../..', import.meta.url));
const ENGINE_SOURCES = fileURLToPath(new URL('../../engine', import.meta.url));
const ENGINE_OUTPUT = fileURLToPath(new URL('../src/engine-room/wasm-build/wasm_bridge_bg.wasm', import.meta.url));

export function engineBuild(): Plugin {
  let profile: '--dev' | '--release' = '--release';

  return {
    name: 'engine-build',
    configResolved(config) {
      profile = config.command === 'serve' ? '--dev' : '--release';
    },
    buildStart() {
      if (process.env.ENGINE_PREBUILT === '1') {
        if (!existsSync(ENGINE_OUTPUT)) {
          this.error('engine build: ENGINE_PREBUILT=1, but there is no engine in src/engine-room/wasm-build/');
        }
        return;
      }
      const built = spawnSync('cargo', ['chores', 'wasm', profile], { cwd: REPOSITORY, stdio: 'inherit' });
      if (built.error !== undefined) this.error(`engine build: cannot run cargo (${built.error.message})`);
      if (built.status !== 0) this.error('engine build: `cargo chores wasm` failed');
    },
    configureServer(server) {
      let running = false;
      let again = false;
      const rebuild = () => {
        if (running) {
          again = true;
          return;
        }
        running = true;
        const child = spawn('cargo', ['chores', 'wasm', '--dev'], { cwd: REPOSITORY, stdio: 'inherit' });
        child.on('close', (status) => {
          running = false;
          if (status === 0) server.ws.send({ type: 'full-reload' });
          else server.config.logger.error('engine build: `cargo chores wasm --dev` failed; the page keeps the last engine');
          if (again) {
            again = false;
            rebuild();
          }
        });
      };
      server.watcher.add(ENGINE_SOURCES);
      server.watcher.on('change', (path) => {
        if (path.startsWith(ENGINE_SOURCES) && path.endsWith('.rs')) rebuild();
      });
    },
  };
}
