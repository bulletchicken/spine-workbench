import { StrictMode, type ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import { AppFrame } from './app-shell/AppFrame';
import { pickEngine } from './engine-room/pick-engine';
import { EngineProvider } from './engine-room/use-engine';
import './look-and-feel/base.css';
import { preferences } from './view-state/preferences';

// What the page mounts: the frame, on the engine chosen for this page.
//
// In development the address can ask for more, and a production build
// holds none of it, because each branch below, and with it the only
// import of what it reaches, is dropped when the build is made:
//
//   ?gallery          every part in every state, in place of the app
//   ?scene=<name>     the app put in a named state from the sample case
//   ?motion=reduced   reduced motion for this visit, whatever the system says

const root = document.getElementById('root');
if (root === null) throw new Error('index.html has no #root element');
const page = createRoot(root);
const asked = new URLSearchParams(window.location.search);

async function start(): Promise<void> {
  const engine = await pickEngine(window.location.search);
  const onTheEngine = (screen: ReactNode) => {
    page.render(
      <StrictMode>
        <EngineProvider engine={engine}>{screen}</EngineProvider>
      </StrictMode>,
    );
  };

  if (import.meta.env.DEV) {
    if (asked.get('motion') === 'reduced') preferences.getState().setMotion('reduced', false);
    if (asked.has('gallery')) {
      const { Gallery } = await import('./gallery/Gallery');
      onTheEngine(<Gallery />);
      return;
    }
  }

  onTheEngine(<AppFrame />);

  if (import.meta.env.DEV) {
    const scene = asked.get('scene');
    if (scene !== null) {
      const { enterScene } = await import('./app-shell/dev-scenes');
      await enterScene(scene, { engine });
    }
  }
}

void start();
