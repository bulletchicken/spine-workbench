import type { GalleryPage } from '@/gallery/gallery-page';
import type { EngineInfo } from '@/shapes';
import './app-shell.gallery.css';
import { StatusLine } from './StatusLine';
import type { EngineStatus } from './use-engine-status';

const INFO: EngineInfo = { engineVersion: '0.1.0', shapesFingerprint: 'gallery', caseFormat: 'spine-case', caseVersion: 1, trainedSideFinder: false };
const READY: EngineStatus = { status: 'ready', info: INFO };

const page: GalleryPage = {
  area: 'app-shell',
  part: 'StatusLine',
  states: [
    {
      name: 'First launch: the claim and the count',
      show: () => (
        <div className="shell-gallery-foot">
          <StatusLine engine={READY} position={null} pixelSize={null} sent={0} />
        </div>
      ),
    },
    {
      name: 'The pointer on a film with a size',
      show: () => (
        <div className="shell-gallery-foot">
          <StatusLine engine={READY} position={{ x: 212, y: 640, unit: 'mm' }} pixelSize="0.45" sent={0} />
        </div>
      ),
    },
    {
      name: 'The pointer on a film with no size: pixels, and it says so',
      show: () => (
        <div className="shell-gallery-foot">
          <StatusLine engine={READY} position={{ x: 471, y: 1422, unit: 'px' }} pixelSize={null} sent={0} />
        </div>
      ),
    },
    {
      name: 'The engine did not start: said in words, for as long as it is so',
      show: () => (
        <div className="shell-gallery-foot">
          <StatusLine engine={{ status: 'failed', error: { code: 'engine-load-failed', message: 'the engine did not start', detail: null } }} position={null} pixelSize={null} sent={0} />
        </div>
      ),
    },
  ],
};

export default page;
