# Sample films on this machine

`on-this-machine/` is ignored by git. It may hold up to six front X-ray films named
`sample-film-1.jpg` through `sample-film-6.jpg` (JPEG or PNG), with no metadata.
Nothing in it is ever committed. The screen shows only a sample number, never a file name.

`pnpm build` excludes these films, even when the folder holds them. `pnpm dev` includes them,
but the stand-in engine offers only the made-up case. Use `?engine=real` to use local sample films.

`pnpm --filter ./app films-on-this-machine` deliberately builds with the films into the separate,
ignored `app/films-on-this-machine/dist/` folder and serves it on this computer. That copy uses the real
engine and the same content policy response header as `pnpm --filter ./app preview`.
Never publish it. The ordinary production output stays in `app/dist/`.

The sample control goes through the local films in number order, then the four-film made-up case,
then round again. With no local films it always offers the made-up case. Your own films replace
the sample before they are taken in. Film bytes are inlined in the app's script; opening a sample
needs no request.
