// The content policy: the browser's own guard that nothing leaves the
// machine. The tag and header policies are defined here.
//
// It is delivered twice. Production builds carry it as the first element
// of index.html's <head>, and the build fails if it is not there. `vite
// preview` also sends it as a response header on every file, because a
// dedicated worker takes its policy from its own response and ignores the
// page's meta tag. The header adds frame-ancestors 'self', which only a
// header can enforce: other sites cannot frame the app. Its distinct text
// also lets the refusal counter tell the two policies apart. A host
// serving the built app must send CONTENT_POLICY_HEADER on every file.
//
// Development has no policy: hot reload needs inline code. So every policy
// test runs against the production build.
//
// 'wasm-unsafe-eval' lets the browser compile WebAssembly without allowing
// JavaScript eval. Workers are emitted as real files, so worker-src needs
// no blob:. Images may be blob: or data: URLs made inside the page.

import type { Plugin } from 'vite';

export const CONTENT_POLICY = [
  "default-src 'self'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "style-src 'self'",
  "img-src 'self' blob: data:",
  "font-src 'self'",
  "connect-src 'self'",
  "worker-src 'self'",
  "frame-src 'none'",
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
].join('; ');

export const CONTENT_POLICY_HEADER = `${CONTENT_POLICY}; frame-ancestors 'self'`;

export const POLICY_META = `<meta http-equiv="Content-Security-Policy" content="${CONTENT_POLICY}">`;

/** True when the policy is the first element in the page's <head>, exactly. */
export function policyComesFirst(html: string): boolean {
  const head = /<head[^>]*>\s*(<meta\b[^>]*>)/i.exec(html);
  return head?.[1] === POLICY_META;
}

export function contentPolicy(): Plugin {
  return {
    name: 'content-policy',
    enforce: 'post',
    transformIndexHtml: {
      order: 'post',
      handler(html, context) {
        if (context.server !== undefined) return html;
        return html.replace(/<head([^>]*)>/i, `<head$1>\n    ${POLICY_META}`);
      },
    },
    generateBundle: {
      // After Vite has written index.html into the bundle.
      order: 'post',
      handler(_options, bundle) {
        const page = bundle['index.html'];
        if (page?.type !== 'asset' || typeof page.source !== 'string') {
          this.error('content policy: the build wrote no index.html');
        }
        if (!policyComesFirst(page.source)) {
          this.error('content policy: index.html must start its <head> with the content policy meta tag');
        }
      },
    },
    configurePreviewServer(server) {
      server.middlewares.use((_request, response, next) => {
        response.setHeader('Content-Security-Policy', CONTENT_POLICY_HEADER);
        next();
      });
    },
  };
}
