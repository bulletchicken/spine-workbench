# 'wasm-unsafe-eval' and the policy header for workers

**Decided:** the content policy's `script-src` is `'self'
'wasm-unsafe-eval'`, and the policy is delivered twice: as the first
element of the production page's `<head>`, and as a response header on
every file the preview server sends.

```
default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self';
img-src 'self' blob: data:; font-src 'self'; connect-src 'self';
worker-src 'self'; frame-src 'none'; object-src 'none'; base-uri 'none';
form-action 'none'
```

## Why

- Without `'wasm-unsafe-eval'`, browsers refuse to compile WebAssembly
  under `script-src 'self'`. Tried on the production build: with it
  removed, the engine does not compile and the page shows the
  engine-load-failed wording. The keyword allows WebAssembly compilation
  only; JavaScript `eval` and `new Function` stay blocked, in the page and
  in the workers.
- A dedicated worker takes its policy from its own script's response
  and ignores the page's meta tag. So the header is what protects the
  workers; the meta tag protects the page wherever the app is hosted.
- `worker-src 'self'` with no `blob:`: Vite emits workers as real files,
  so nothing needs blob workers, and refusing them closes a way around
  the policy. `frame-src 'none'` is stricter than the default needs.

## Consequences

- `app/build-plugins/content-policy.ts` holds the only copy of the
  string. The build fails if the page does not start its `<head>` with
  it, and `vite preview` sends it as a header on every file.
- Development mode has no policy (hot reload needs inline code), so every
  policy test runs on the production build.
- A host serving the built app must send the same header for the workers
  to be covered. Where a host cannot, the lint rule, the dependency
  seams and the request-recording test still hold.
- `content-policy.spec.ts` checks the exact string in both places, eval
  blocked in the page and in each worker, an outside fetch refused by the
  policy itself, and no blob: workers.
