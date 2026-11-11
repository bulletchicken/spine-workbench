import { Component, type ReactNode } from 'react';
import type { MotionMode } from '@/look-and-feel/data-attributes';
import type { FoundPage } from './find-pages';
import type { GalleryPage } from './gallery-page';
import './GalleryStage.css';

/** A state is drawn as a component of its own, so a state that keeps something in a hook keeps it to itself. */
function Shown({ state }: { state: GalleryPage['states'][number] }) {
  return state.show();
}

/** One state that throws does not take the rest of the page with it. */
class PlateBoundary extends Component<{ children: ReactNode }, { failure: string | null }> {
  override state: { failure: string | null } = { failure: null };

  static getDerivedStateFromError(error: unknown): { failure: string } {
    return { failure: error instanceof Error ? error.message : String(error) };
  }

  override render(): ReactNode {
    if (this.state.failure === null) return this.props.children;
    return (
      <p className="gallery-plate-failure" role="alert">
        This state threw while drawing: {this.state.failure}
      </p>
    );
  }
}

interface GalleryStageProps {
  found: FoundPage;
  motion: MotionMode;
}

/**
 * One gallery page: its part in each state, each on a plate under its
 * name. The stage is its own app root as far as CSS can tell: it carries
 * `data-motion`, as the app's frame does.
 *
 * A plate carries `data-gallery-state`, so a script can find a state by
 * name and take its picture.
 */
export function GalleryStage({ found, motion }: GalleryStageProps) {
  const { page } = found;
  return (
    <main className="gallery-stage" data-motion={motion}>
      <header className="gallery-stage-head">
        <span className="gallery-stage-area">{page.area}</span>
        <h1 className="gallery-stage-part">{page.part}</h1>
      </header>
      <div className="gallery-plates">
        {page.states.map((state) => (
          <section key={state.name} className="gallery-plate" data-gallery-state={state.name}>
            <h2 className="gallery-plate-name">{state.name}</h2>
            <div className="gallery-plate-body">
              <PlateBoundary>
                <Shown state={state} />
              </PlateBoundary>
            </div>
          </section>
        ))}
      </div>
    </main>
  );
}
