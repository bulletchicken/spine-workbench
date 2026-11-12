# 'wasm-unsafe-eval' and the policy header for workers

**Decided:** the content policy's `script-src` is `'self'
'wasm-unsafe-eval'`, and the policy is delivered twice: as the first
element of the production page's `<head>`, and as a response header on
every file the preview server sends. The header adds `frame-ancestors
'self'` to the tag's policy below.

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
- `frame-ancestors 'self'` stops other sites framing the app. It works
  only in a header, so the tag keeps the shared rules and the header adds
  this one. Their distinct texts let the refusal counter count each
  policy's reports separately and take the largest count of attempts.
- `worker-src 'self'` with no `blob:`: Vite emits workers as real files,
  so nothing needs blob workers, and refusing them closes a way around
  the policy. `frame-src 'none'` is stricter than the default needs.

## Consequences

- `app/build-plugins/content-policy.ts` defines `CONTENT_POLICY` for the
  tag and `CONTENT_POLICY_HEADER` for the header. The build fails if the
  page does not start its `<head>` with the tag, and `vite preview` sends
  the header on every file.
- Development mode has no policy (hot reload needs inline code), so every
  policy test runs on the production build.
- A host serving the built app must send `CONTENT_POLICY_HEADER` on
  every file for the workers and framing rule to be covered. Repeating
  the tag's exact text instead keeps the network guard but makes the
  blocked count an upper bound. Where a host cannot send a header, the
  lint rule, the dependency seams and the request-recording test still
  hold.
- `content-policy.spec.ts` checks the exact tag and header texts, eval
  blocked in the page and in each worker, an outside fetch refused by the
  policy itself, and no blob: workers.
