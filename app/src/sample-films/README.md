# Sample films on this machine

`on-this-machine/` is ignored by git. It may hold up to six front X-ray films named
`sample-film-1.jpg` through `sample-film-6.jpg` (JPEG or PNG), with no metadata.
Nothing in it is ever committed. The screen shows only a sample number, never a file name.

`pnpm build` excludes these films, even when the folder holds them. `pnpm dev` includes them,
and when there are any it opens on the real engine, because the stand-in cannot read them.
`?engine=stand-in`, a `?scene` or the `?gallery` keep the stand-in, which offers only the made-up case.

`pnpm --filter ./app films-on-this-machine` deliberately builds with the films into the separate,
ignored `app/films-on-this-machine/dist/` folder and serves it on this computer. That copy uses the real
engine and the same content policy response header as `pnpm --filter ./app preview`.
Never publish it. The ordinary production output stays in `app/dist/`.

The sample control goes through the local films in number order, then round again; the four-film
made-up case is not among them. With no local films it always offers the made-up case. Your own films replace
the sample before they are taken in. Film bytes are inlined in the app's script; opening a sample
needs no request.

## One real case

A local `on-this-machine/case/` folder may hold `front.png`, `side.png`, `bend-left.png`
and `bend-right.png` (each also accepts `.jpg` or `.jpeg`). Only `front` is required.
It is offered first as “Use the real sample case”, before the numbered front films. Each film goes straight to its named place and uses the ordinary finders and fit.
Without this folder the sample control is unchanged. These files are inlined only in development
and the deliberate machine build, never in an ordinary production build.

An optional `case/case.json` may contain:

```json
{
  "sideFaces": "left",
  "credit": "Source name — licence and attribution"
}
```

`sideFaces` is `"left"` or `"right"`, the side of the picture the patient faces in the side film.
It answers the facing question; the question stays visible and can be changed.
`credit` is one line shown unchanged after “Films: ” in the stage caption, or in Place’s sheet
notes. A licence’s credit line belongs in `credit`. The credit leaves when these films are
replaced or the case is started over. Neither field is required; never put film names in `credit`.
