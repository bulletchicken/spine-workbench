# Map

"I want to change X" → open Y, in the order a film travels. Searching for
"center"? We spell it "centre" (and "colour", "grey", "normalise").

`cargo chores map --check` fails CI if any path on this page does not
exist. The engine half is complete; the app half grows as the screens are
built.

## The repository

| I want to change… | Open |
|---|---|
| what the project is and how to run it | `README.md` |
| how we branch, commit and review | `CONTRIBUTING.md` |
| the licence and third-party credits | `LICENSE`, `NOTICE` |
| the licence text of each bundled typeface and bone model | `third-party-licences/` |
| the Rust workspace, its pinned dependencies, the build profiles | `Cargo.toml`, `Cargo.lock` |
| the Rust toolchain | `rust-toolchain.toml` |
| the Rust formatting and lint rules (no clocks, threads, files, std float maths) | `rustfmt.toml`, `clippy.toml` |
| SIMD for the browser build, the `cargo chores` and `cargo spine` aliases | `.cargo/config.toml` |
| the root scripts: dev, engine, build, check, test, browser, verify, score, and the hook set-up | `package.json` |
| which packages pnpm manages | `pnpm-workspace.yaml`, `pnpm-lock.yaml` |
| the Node version and engine-strict | `.node-version`, `.npmrc` |
| the TypeScript compiler options every project shares | `tsconfig.base.json` |
| the network ban, the import boundaries and the lint rules | `eslint.config.js` |
| what git ignores and how it treats files | `.gitignore`, `.gitattributes`, `.editorconfig` |
| the pull request template | `.github/pull_request_template.md` |
| CI for the engine | `.github/workflows/rust.yml` |
| CI for the app and the browser tests | `.github/workflows/app.yml` |
| CI for commit messages and pull request text | `.github/workflows/commit-hygiene.yml` |

## Repository upkeep (`chores/`)

| I want to change… | Open |
|---|---|
| the list of `cargo chores` tasks | `chores/src/main.rs` |
| how the engine is built for the browser | `chores/src/wasm.rs` |
| how the TypeScript shapes, the JSON Schema and the fingerprint are written | `chores/src/shapes.rs` |
| which crate may reach which (browser, network and tool crates) | `chores/src/seams.rs` |
| the wasm-bindgen crate and tool version check | `chores/src/versions.rs` |
| this page's path check | `chores/src/map.rs` |
| the commit message rules | `chores/check-commit-message.sh` |
| the git hook that runs them on every commit | `chores/git-hooks/commit-msg` |

## Bone shapes (`bone-prep/`)

| I want to change… | Open |
|---|---|
| where the vertebra shapes come from, their licence, and what was done to them | `bone-prep/README.md` |
| how a vertebra's body is found and the bone moved into its own frame | `bone-prep/prepare_bones.py` |

## The engine (`engine/`)

The crate map, the dependency rules and the patient frame in one picture:
`engine/README.md`.

### Shared shapes (`engine/shapes/`)

| I want to change… | Open |
|---|---|
| the list of shapes, the case format and version, the fingerprint | `engine/shapes/src/lib.rs`, `engine/shapes/build.rs` |
| points, boxes and the pixel-space rule | `engine/shapes/src/pixels.rs` |
| a film's kind, stored image, kept DICOM tags and facts | `engine/shapes/src/film.rs` |
| scale, quarter turns, facing and confirmations | `engine/shapes/src/film_geometry.rs` |
| levels and their order | `engine/shapes/src/levels.rs` |
| marks and their names | `engine/shapes/src/marks.rs` |
| a vertebra on a film, a film's marks, and swapping their left/right names after a facing change | `engine/shapes/src/vertebra.rs` |
| C7, the top of the sacrum, hip centres | `engine/shapes/src/landmarks.rs` |
| confidence, its bands and reasons | `engine/shapes/src/confidence.rs` |
| level naming, counts, each count's levels and names, and flags | `engine/shapes/src/naming.rs` |
| what the auto pass takes and returns | `engine/shapes/src/auto_pass.rs` |
| the watch frames | `engine/shapes/src/watch_frames.rs` |
| tool previews and the wand and pin inputs and outputs | `engine/shapes/src/toolbox.rs` |
| case check findings | `engine/shapes/src/checks.rs` |
| made-up recipes, answer keys and comparisons | `engine/shapes/src/made_up.rs` |
| the joint fit's inputs, poses, uncertainty and stopping reason | `engine/shapes/src/spine_fit.rs` |
| engine info | `engine/shapes/src/engine_info.rs` |
| error codes | `engine/shapes/src/errors.rs` |
| the saved case record | `engine/shapes/src/case_file_v1.rs` |
| strict reading and canonical writing of case.json | `engine/shapes/src/reading.rs` |
| the strict-reading, mark-name, golden-file and side-swap tests | `engine/shapes/tests/strict_reading.rs`, `engine/shapes/tests/mark_names_are_exact.rs`, `engine/shapes/tests/golden_case_files.rs`, `engine/shapes/tests/side_swap.rs` |
| which watch frames may be thinned | `engine/shapes/tests/which_frames_may_be_thinned.rs` |

### Image filters (`engine/darkroom/`)

| I want to change… | Open |
|---|---|
| the image type and where it sits on the film | `engine/darkroom/src/grey_image.rs` |
| percentile windows, 16-bit to float | `engine/darkroom/src/normalise.rs` |
| Gaussian and box blurs | `engine/darkroom/src/blur.rs` |
| gradients | `engine/darkroom/src/gradients.rs` |
| ridge and valley measures | `engine/darkroom/src/hessian.rs` |
| the 2D FFT | `engine/darkroom/src/fft.rs` |
| working-resolution copies and their map back | `engine/darkroom/src/pyramid.rs` |
| sampling along a curve and its normals | `engine/darkroom/src/sampling.rs` |
| flood fill, connected parts, outlines | `engine/darkroom/src/regions.rs` |
| preview tiles for watch frames | `engine/darkroom/src/preview.rs` |
| the log-Gabor filter bank | `engine/darkroom/src/log_gabor.rs` |
| phase congruency and phase symmetry | `engine/darkroom/src/phase_congruency.rs` |
| the ring detector | `engine/darkroom/src/radial_symmetry.rs` |

### The model of a vertebra (`engine/forward-model/`)

| I want to change… | Open |
|---|---|
| the patient frame | `engine/forward-model/src/patient_frame.rs` |
| the 13 numbers and their signs | `engine/forward-model/src/vertebra_params.rs` |
| rotation vector and matrix | `engine/forward-model/src/rotation.rs` |
| endplate centres, tilts and rims | `engine/forward-model/src/endplates.rs` |
| the closed-form corner | `engine/forward-model/src/corners.rs` |
| default pedicle positions | `engine/forward-model/src/pedicles.rs` |
| cameras: patient millimetres to film pixels | `engine/forward-model/src/cameras/mod.rs`, `engine/forward-model/src/cameras/parallel.rs` |
| a vertebra and a camera to marks | `engine/forward-model/src/project.rs` |

### Made-up spines (`engine/made-up-spines/`)

| I want to change… | Open |
|---|---|
| the entry point: recipe and seed to films and answer key | `engine/made-up-spines/src/lib.rs` |
| the named recipes | `engine/made-up-spines/src/recipes.rs` |
| the seeded random numbers | `engine/made-up-spines/src/seeded.rs` |
| the curve generators | `engine/made-up-spines/src/curves/bump_curves.rs`, `engine/made-up-spines/src/curves/spline_curves.rs` |
| stacking from L5 | `engine/made-up-spines/src/stack_from_l5.rs` |
| how films are painted | `engine/made-up-spines/src/film_painter/mod.rs` |
| bodies and pedicle rings | `engine/made-up-spines/src/film_painter/bodies.rs` |
| ribs, shoulders, skull, pelvis | `engine/made-up-spines/src/film_painter/surroundings.rs` |
| the lead marker, ruler and calibration ball | `engine/made-up-spines/src/film_painter/markers.rs` |
| blur, noise and quantisation | `engine/made-up-spines/src/film_painter/detector.rs` |
| click noise on true marks | `engine/made-up-spines/src/click_noise.rs` |
| the answer key | `engine/made-up-spines/src/answer_key.rs` |

### Joint fit (`engine/joint-fit/`)

| I want to change… | Open |
|---|---|
| fitting front and side marks as one spine | `engine/joint-fit/src/lib.rs` |
| the joint fit's browser export | `engine/wasm-bridge/src/exports/joint_fit.rs` |

### Reading films (`engine/film-reader/`)

| I want to change… | Open |
|---|---|
| the entry point: bytes to a decoded film | `engine/film-reader/src/lib.rs` |
| what a decoded film holds | `engine/film-reader/src/decoded_film.rs` |
| PNG, JPEG or DICOM, from the bytes | `engine/film-reader/src/sniff.rs` |
| PNG and JPEG decoding | `engine/film-reader/src/png_jpeg.rs` |
| the DICOM tag keep-list | `engine/film-reader/src/dicom/keep_list.rs` |
| DICOM compressions | `engine/film-reader/src/dicom/pixel_data.rs` |
| rescale, bits stored, MONOCHROME1 | `engine/film-reader/src/dicom/grey_levels.rs` |
| DICOM spacing tags | `engine/film-reader/src/dicom/scale_tags.rs` |
| the half-float copy for the GPU | `engine/film-reader/src/display_pixels.rs` |
| thumbnails | `engine/film-reader/src/thumbnail.rs` |
| test DICOM writers | `engine/film-reader/src/test_support.rs` |

### Film geometry (`engine/film-geometry/`)

| I want to change… | Open |
|---|---|
| the entry point: sensing a film's geometry | `engine/film-geometry/src/lib.rs` |
| the coarse bone trace | `engine/film-geometry/src/rough_trace.rs` |
| scale from tags, a ruler or a known object | `engine/film-geometry/src/scale/from_tags.rs`, `engine/film-geometry/src/scale/from_ruler.rs`, `engine/film-geometry/src/scale/from_known_object.rs` |
| upright and upside-down | `engine/film-geometry/src/upright.rs` |
| side-film facing | `engine/film-geometry/src/side_facing.rs` |
| the lead L/R marker | `engine/film-geometry/src/lead_marker.rs` |

### The auto pass (`engine/auto-pass/`)

| I want to change… | Open |
|---|---|
| the stages and how they hand over | `engine/auto-pass/src/lib.rs`, `engine/auto-pass/src/handoffs.rs` |
| a tunable weight | `engine/auto-pass/src/settings.rs` |
| stage 1: edges, ridges and gaps, pedicle rings | `engine/auto-pass/src/evidence/edges.rs`, `engine/auto-pass/src/evidence/ridges.rs`, `engine/auto-pass/src/evidence/pedicle_rings.rs` |
| stage 2: the centre line | `engine/auto-pass/src/centre_line/cost_map.rs`, `engine/auto-pass/src/centre_line/cheapest_path.rs`, `engine/auto-pass/src/centre_line/smooth.rs` |
| stage 3: the unrolled strip and its barcode | `engine/auto-pass/src/strip/unroll.rs`, `engine/auto-pass/src/strip/barcode.rs` |
| stage 4: the chain search | `engine/auto-pass/src/chain_search/gap_score.rs`, `engine/auto-pass/src/chain_search/height_prior.rs`, `engine/auto-pass/src/chain_search/two_views.rs`, `engine/auto-pass/src/chain_search/dynamic_program.rs` |
| stage 5: naming from both ends | `engine/auto-pass/src/level_naming/count_from_top.rs`, `engine/auto-pass/src/level_naming/count_from_bottom.rs`, `engine/auto-pass/src/level_naming/disagreement.rs` |
| stage 6: confidence | `engine/auto-pass/src/confidence/per_vertebra.rs` |
| corners, pedicles and endplate lines on the film | `engine/auto-pass/src/points/back_to_film.rs`, `engine/auto-pass/src/points/front.rs`, `engine/auto-pass/src/points/side.rs`, `engine/auto-pass/src/points/bending.rs` |
| C7, sacrum top, hip centres | `engine/auto-pass/src/landmarks/c7.rs`, `engine/auto-pass/src/landmarks/sacrum_top.rs`, `engine/auto-pass/src/landmarks/hip_centres.rs` |
| the steppers and their frames | `engine/auto-pass/src/watch/film_pass.rs`, `engine/auto-pass/src/watch/chain_pass.rs`, `engine/auto-pass/src/watch/frames.rs` |
| one-call native entry points | `engine/auto-pass/src/run.rs` |

### The adjusting tools' maths (`engine/toolbox/`)

| I want to change… | Open |
|---|---|
| the region wand | `engine/toolbox/src/region_wand/hover.rs`, `engine/toolbox/src/region_wand/local_edges.rs`, `engine/toolbox/src/region_wand/grow.rs`, `engine/toolbox/src/region_wand/carve.rs`, `engine/toolbox/src/region_wand/refit.rs` |
| the boundary wand | `engine/toolbox/src/boundary_wand/hover.rs`, `engine/toolbox/src/boundary_wand/split.rs`, `engine/toolbox/src/boundary_wand/merge.rs`, `engine/toolbox/src/boundary_wand/renumber.rs` |
| pins and the re-solve around them | `engine/toolbox/src/pins/re_solve.rs` |

### Checks and the case file

| I want to change… | Open |
|---|---|
| the list of case checks | `engine/case-checks/src/lib.rs` |
| every vertebra present; in order; off by one; films agree; confirmations | `engine/case-checks/src/presence.rs`, `engine/case-checks/src/order.rs`, `engine/case-checks/src/off_by_one.rs`, `engine/case-checks/src/films_agree.rs`, `engine/case-checks/src/confirmations.rs` |
| what a case zip may hold | `engine/case-file/src/layout.rs` |
| films as PNG | `engine/case-file/src/film_png.rs` |
| packing and unpacking | `engine/case-file/src/pack.rs`, `engine/case-file/src/unpack.rs` |
| zip-level refusals and limits | `engine/case-file/src/strict.rs` |
| case file versions | `engine/case-file/src/versions/mod.rs`, `engine/case-file/src/versions/v1.rs` |

### Scoring (`engine/scorekeeper/`)

| I want to change… | Open |
|---|---|
| a dataset reader | `engine/scorekeeper/src/datasets/aasce_2019.rs`, `engine/scorekeeper/src/datasets/vertebrae_keypoints_50.rs`, `engine/scorekeeper/src/datasets/hand_marked.rs`, `engine/scorekeeper/src/datasets/made_up.rs` |
| a metric | `engine/scorekeeper/src/metrics/corner_error.rs`, `engine/scorekeeper/src/metrics/vertebrae_found.rs`, `engine/scorekeeper/src/metrics/level_naming.rs`, `engine/scorekeeper/src/metrics/cobb_from_corners.rs`, `engine/scorekeeper/src/metrics/clicks_to_good_outline.rs` |
| the simulated user and the by-hand baseline | `engine/scorekeeper/src/simulated_user/region_wand_clicker.rs`, `engine/scorekeeper/src/simulated_user/boundary_clicker.rs`, `engine/scorekeeper/src/simulated_user/by_hand_layout.rs` |
| a scoring suite | `engine/scorekeeper/src/suites/ci.rs`, `engine/scorekeeper/src/suites/auto_pass_front.rs`, `engine/scorekeeper/src/suites/auto_pass_side.rs`, `engine/scorekeeper/src/suites/two_films_voting.rs`, `engine/scorekeeper/src/suites/region_wand.rs` |
| reports | `engine/scorekeeper/src/report/markdown.rs`, `engine/scorekeeper/src/report/json.rs`, `engine/scorekeeper/src/report/overlay_svg.rs` |

### The `spine` command (`engine/command-line/`)

| I want to change… | Open |
|---|---|
| the subcommands and their arguments | `engine/command-line/src/main.rs` |
| `spine make-up`, `open`, `pass`, `frames`, `case`, `score` | `engine/command-line/src/commands/make_up.rs`, `engine/command-line/src/commands/open.rs`, `engine/command-line/src/commands/pass.rs`, `engine/command-line/src/commands/frames.rs`, `engine/command-line/src/commands/case.rs`, `engine/command-line/src/commands/score.rs` |

### The door to the browser (`engine/wasm-bridge/`)

| I want to change… | Open |
|---|---|
| the start hook | `engine/wasm-bridge/src/lib.rs` |
| `FilmDesk` | `engine/wasm-bridge/src/film_desk.rs` |
| how values cross into JavaScript | `engine/wasm-bridge/src/crossing.rs` |
| thrown errors and the last panic | `engine/wasm-bridge/src/faults.rs` |
| the export of one crate (same name as the crate) | `engine/wasm-bridge/src/exports/mod.rs`, `engine/wasm-bridge/src/exports/info.rs`, `engine/wasm-bridge/src/exports/shapes.rs`, `engine/wasm-bridge/src/exports/film_reader.rs`, `engine/wasm-bridge/src/exports/film_geometry.rs`, `engine/wasm-bridge/src/exports/auto_pass.rs`, `engine/wasm-bridge/src/exports/toolbox.rs`, `engine/wasm-bridge/src/exports/case_checks.rs`, `engine/wasm-bridge/src/exports/case_file.rs`, `engine/wasm-bridge/src/exports/made_up_spines.rs`, `engine/wasm-bridge/src/exports/scorekeeper.rs` |

## The app (`app/`)

The folders and import rules in one picture: `app/README.md`.

| I want to change… | Open |
|---|---|
| the page, Vite, TypeScript and test setup | `app/index.html`, `app/vite.config.ts`, `app/tsconfig.json`, `app/vitest.config.ts`, `app/package.json` |
| the content policy (the only copy) | `app/build-plugins/content-policy.ts` |
| the engine build inside Vite | `app/build-plugins/engine-build.ts` |
| what the page mounts | `app/src/main.tsx` |
| the shared types and the mark and level lists | `app/src/shapes/index.ts`, `app/src/shapes/mark-names.ts` |
| the engine client screens use | `app/src/engine-room/engine-client.ts` |
| the job table | `app/src/engine-room/job-table.ts` |
| the worker messages | `app/src/engine-room/messages.ts` |
| the one fetch: loading the engine | `app/src/engine-room/load-engine.ts` |
| how a worker boots | `app/src/engine-room/workers/boot.ts`, `app/src/engine-room/workers/film-worker/film.worker.ts`, `app/src/engine-room/workers/spine-worker/spine.worker.ts` |
| what the stand-in's story must hold | `app/src/sample-case/story-shape.ts` |
| the step contract | `app/src/steps/step-contract.ts` |
| the tool contract | `app/src/toolbox/tool-contract.ts` |
| the film-to-screen transform | `app/src/film-viewer/film-space.ts` |
| the vertebra shapes the 3D view draws, and how they are read | `app/public/bones/vertebrae.json`, `app/public/bones/vertebrae.bin`, `app/src/spine-3d/bones/read-bones.ts` |
| the case edits | `app/src/case-state/case-edits.ts` |
| the design's tokens, unchanged | `app/src/look-and-feel/tokens.css` |
| a duration the design names that is not a token | `app/src/look-and-feel/named-timings.css` |
| the one stylesheet, which imports the shared ones in order | `app/src/look-and-feel/base.css` |
| the values of the data attributes that carry state to CSS | `app/src/look-and-feel/data-attributes.ts` |
| what a gallery page holds | `app/src/gallery/gallery-page.ts` |
| the preview notice, error wording, evidence entries, the one word for each idea | `app/src/words/preview-notice.ts`, `app/src/words/faults.ts`, `app/src/words/evidence-ledger.ts`, `app/src/words/glossary.ts` |
| the rule against outcome claims in the wording | `app/src/words/never-claim.test.ts` |

## The proof (`proving-ground/`)

The proof ladder: `proving-ground/README.md`.

| I want to change… | Open |
|---|---|
| a golden case file or its verdict | `proving-ground/known-answers/case-files/verdicts.json` |
| the known answer for swapping left/right names | `proving-ground/known-answers/side-swap.json` |
| the real engine in Node | `proving-ground/parity/contract.test.ts` |
| the no-network browser test | `proving-ground/in-the-browser/no-network/nothing-leaves.spec.ts` |
| the content-policy browser test | `proving-ground/in-the-browser/no-network/content-policy.spec.ts` |
| the proof that the no-network test sees a worker's violations | `proving-ground/in-the-browser/no-network/worker-violations-are-seen.spec.ts` |
| the reviewed list of addresses in the build | `proving-ground/in-the-browser/no-network/no-outside-urls-in-build.test.ts` |
| the request ledger and other test rigging | `proving-ground/in-the-browser/test-rig/request-ledger.ts`, `proving-ground/in-the-browser/test-rig/policy-violations.ts`, `proving-ground/in-the-browser/test-rig/built-app.ts`, `proving-ground/in-the-browser/test-rig/hold-still.ts` |
| which browsers run, and the preview server | `proving-ground/playwright.config.ts` |

## The explanations (`docs/`)

| I want to change… | Open |
|---|---|
| what is where in the docs | `docs/README.md` |
| a shared convention | `docs/conventions/pixel-space.md`, `docs/conventions/patient-frame.md`, `docs/conventions/signs-and-wedges.md`, `docs/conventions/mark-and-level-names.md`, `docs/conventions/naming-in-code.md` |
| the description of a frozen contract | `docs/contracts/engine-api.md`, `docs/contracts/worker-messages.md`, `docs/contracts/watch-frames.md`, `docs/contracts/case-file-v1.md` |
| the generated case file schema (via `cargo chores shapes`) | `docs/contracts/case-file-v1.schema.json` |
| a recorded decision | `docs/decisions/` |
