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
| how a vertebra's body and its endplates are found, and the bone moved into its own frame | `bone-prep/prepare_bones.py` |
| the typical spine the one-film fit falls back on: its sources, how it is built, and the file it writes | `bone-prep/typical-spine.md`, `bone-prep/typical_spine.py`, `engine/joint-fit/typical-spine/typical-spine.json` |

## The trained finder's numbers (`finder-prep/`)

| I want to change… | Open |
|---|---|
| what the two scripts need, when to run them, and what they write | `finder-prep/README.md` |
| the weights: what they are, what they were trained on, and the citation they need | `engine/trained-finder/weights/README.md`, `engine/trained-finder/weights/` |
| how the trained networks are packed into the one weight file | `finder-prep/pack_weights.py` |
| how the known answers are recorded from the training code's own pipeline | `finder-prep/make_known_answers.py`, `engine/trained-finder/tests/known-answers/README.md` |

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
| fitted spine readings: angles, curves, fixed-end Cobb, sagittal bends and every pair with version bands | `engine/shapes/src/spine_readings.rs` |
| placement proposals: what goes in, curves, rule votes, level pairs, anchors, rods, what needs a look | `engine/shapes/src/implant_layout.rs` |
| the load bench: batches of runs, stops, estimates, tags, shares and paired counts | `engine/shapes/src/load_bench.rs` |
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
| fitting one front film, with a typical spine for what it cannot show | `engine/joint-fit/src/from_films.rs` |
| finding the curves on the front film, and the largest one choosing the typical side profile | `engine/joint-fit/src/front_curves.rs` |
| the typical twist the largest curve gives its vertebrae | `engine/joint-fit/src/curve_twist.rs` |
| the joint fit's browser export | `engine/wasm-bridge/src/exports/joint_fit.rs` |

### Fitted spine readings (`engine/spine-readings/`)

| I want to change… | Open |
|---|---|
| fitted angles, curves and their version bands | `engine/spine-readings/README.md`, `engine/spine-readings/src/lib.rs` |
| its browser export | `engine/wasm-bridge/src/exports/spine_readings.rs` |

### Placement proposals (`engine/implant-layout/`)

| I want to change… | Open |
|---|---|
| the entry point and its refusals | `engine/implant-layout/src/lib.rs` |
| the published tables it reads | `engine/implant-layout/src/evidence.rs` |
| placement classification over the shared measurement versions | `engine/implant-layout/src/readings.rs` |
| Lenke curve type, modifiers and structural tests | `engine/implant-layout/src/curves.rs` |
| the level rules and checks | `engine/implant-layout/src/level_rules.rs` |
| level pairs and the first pair | `engine/implant-layout/src/pairs.rs` |
| pedicle widths | `engine/implant-layout/src/pedicles.rs` |
| screw entry, direction, diameter and length | `engine/implant-layout/src/screws.rs` |
| which pedicles get screws or hooks | `engine/implant-layout/src/patterns.rs` |
| the rods | `engine/implant-layout/src/rods.rs` |
| the placements one step away | `engine/implant-layout/src/alternatives.rs` |
| the whole answer put together | `engine/implant-layout/src/proposal.rs` |
| its browser export | `engine/wasm-bridge/src/exports/implant_layout.rs` |

### The load bench (`engine/load-bench/`)

| I want to change… | Open |
|---|---|
| the entry points and their refusals | `engine/load-bench/src/lib.rs` |
| the published tables it reads | `engine/load-bench/src/evidence.rs` |
| the chain of vertebrae and its joint springs | `engine/load-bench/src/chain.rs`, `engine/load-bench/src/joint_springs.rs` |
| the solve | `engine/load-bench/src/chain_solver.rs` |
| rigid rods, flexible rods and springback | `engine/load-bench/src/rigid_rod.rs`, `engine/load-bench/src/rod_beams.rs`, `engine/load-bench/src/springback.rs` |
| rod moments and stress | `engine/load-bench/src/rod_statics.rs` |
| daily loads, anchor demand, junctions | `engine/load-bench/src/daily_loads.rs`, `engine/load-bench/src/anchor_demand.rs`, `engine/load-bench/src/junctions.rs` |
| a year of walking | `engine/load-bench/src/fatigue.rs` |
| what each version draws, a batch, the comparison | `engine/load-bench/src/versions.rs`, `engine/load-bench/src/batch.rs`, `engine/load-bench/src/compare.rs` |
| its browser export | `engine/wasm-bridge/src/exports/load_bench.rs` |

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
| the film as 8-bit grey, the way it opens on screen | `engine/film-reader/src/eight_bit.rs` |
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

### The trained finder (`engine/trained-finder/`)

| I want to change… | Open |
|---|---|
| what it does, how well, and where it fails | `engine/trained-finder/README.md` |
| the shapes the stages hand on: a peak, a reading, a chain link | `engine/trained-finder/src/outline.rs` |
| reading the compiled-in weights, the chain priors and the framing settings | `engine/trained-finder/src/weights/mod.rs`, `engine/trained-finder/src/weights/table.rs` |
| the film as the network is shown it: through its record, upright, 8-bit, shrunk to 384 by 768 | `engine/trained-finder/src/intake/as_shown.rs`, `engine/trained-finder/src/intake/shrink.rs` |
| the convolution kernel and the other layers | `engine/trained-finder/src/layers/conv.rs`, `engine/trained-finder/src/layers/pointwise.rs`, `engine/trained-finder/src/layers/resample.rs`, `engine/trained-finder/src/layers/group_norm.rs`, `engine/trained-finder/src/layers/activation.rs`, `engine/trained-finder/src/layers/softmax_mean.rs`, `engine/trained-finder/src/grid.rs` |
| the main network, and how one reading is cut into units | `engine/trained-finder/src/spine_net/mod.rs`, `engine/trained-finder/src/spine_net/stepped.rs` |
| peaks and outlines from the network's maps, and how much two outlines overlap | `engine/trained-finder/src/reading/mod.rs`, `engine/trained-finder/src/overlap.rs` |
| the crop for a second reading | `engine/trained-finder/src/framing/mod.rs` |
| the chain: seventeen in order, one gap filled | `engine/trained-finder/src/chain/mod.rs`, `engine/trained-finder/src/chain/costs.rs` |
| the corner patch, the corner network, and how far a corner may move | `engine/trained-finder/src/corners/patch.rs`, `engine/trained-finder/src/corners/corner_net.rs`, `engine/trained-finder/src/corners/settle.rs` |
| the order of the work, its units and its frames | `engine/trained-finder/src/watch/search.rs`, `engine/trained-finder/src/watch/trained_pass.rs` |
| the marks it hands back | `engine/trained-finder/src/to_marks.rs` |
| one-call native entry points | `engine/trained-finder/src/run.rs` |
| the tests against the known answers, and the stepper's tests | `engine/trained-finder/tests/finds_the_vertebrae_on_a_made_up_film.rs`, `engine/trained-finder/tests/spine_net_matches_the_known_answers.rs` |

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
| a scoring suite | `engine/scorekeeper/src/suites/ci.rs`, `engine/scorekeeper/src/suites/auto_pass_front.rs`, `engine/scorekeeper/src/suites/trained_finder_front.rs`, `engine/scorekeeper/src/suites/auto_pass_side.rs`, `engine/scorekeeper/src/suites/two_films_voting.rs`, `engine/scorekeeper/src/suites/region_wand.rs` |
| reports | `engine/scorekeeper/src/report/markdown.rs`, `engine/scorekeeper/src/report/json.rs`, `engine/scorekeeper/src/report/overlay_svg.rs` |

### The `spine` command (`engine/command-line/`)

| I want to change… | Open |
|---|---|
| the subcommands and their arguments | `engine/command-line/src/main.rs` |
| `spine measure`: find, fit and measure front/side films, or measure a made-up spine | `engine/command-line/src/commands/measure.rs` |
| `spine make-up`, `open`, `pass`, `frames`, `case`, `score` | `engine/command-line/src/commands/make_up.rs`, `engine/command-line/src/commands/open.rs`, `engine/command-line/src/commands/pass.rs`, `engine/command-line/src/commands/frames.rs`, `engine/command-line/src/commands/case.rs`, `engine/command-line/src/commands/score.rs` |
| which finder reads the front film in `spine pass` and `spine score`, and how the films' answers are joined, as the app's conductor joins them | `engine/command-line/src/commands/pass.rs` |
| `spine measure`: find, fit and measure front/side films, or measure a made-up spine | `engine/command-line/src/commands/measure.rs` |

### The door to the browser (`engine/wasm-bridge/`)

| I want to change… | Open |
|---|---|
| the start hook | `engine/wasm-bridge/src/lib.rs` |
| `FilmDesk` | `engine/wasm-bridge/src/film_desk.rs` |
| how values cross into JavaScript | `engine/wasm-bridge/src/crossing.rs` |
| thrown errors and the last panic | `engine/wasm-bridge/src/faults.rs` |
| the export of one crate (same name as the crate) | `engine/wasm-bridge/src/exports/mod.rs`, `engine/wasm-bridge/src/exports/info.rs`, `engine/wasm-bridge/src/exports/shapes.rs`, `engine/wasm-bridge/src/exports/film_reader.rs`, `engine/wasm-bridge/src/exports/film_geometry.rs`, `engine/wasm-bridge/src/exports/auto_pass.rs`, `engine/wasm-bridge/src/exports/toolbox.rs`, `engine/wasm-bridge/src/exports/case_checks.rs`, `engine/wasm-bridge/src/exports/case_file.rs`, `engine/wasm-bridge/src/exports/made_up_spines.rs`, `engine/wasm-bridge/src/exports/scorekeeper.rs`, `engine/wasm-bridge/src/exports/trained_finder.rs`, `engine/wasm-bridge/src/exports/implant_layout.rs`, `engine/wasm-bridge/src/exports/load_bench.rs` |

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
| rebuilding a placement changed by hand, on the spine worker | `app/src/engine-room/workers/spine-worker/jobs/rebuild-layout.ts`, `app/src/engine-room/workers/spine-worker/jobs/rebuild-layout.test.ts` |
| the stand-in placement story and its contract check | `app/src/engine-room/stand-in/layout-story.ts`, `app/src/engine-room/stand-in/layout-story.test.ts` |
| the worker messages | `app/src/engine-room/messages.ts` |
| the one fetch: loading the engine | `app/src/engine-room/load-engine.ts` |
| how a worker boots | `app/src/engine-room/workers/boot.ts`, `app/src/engine-room/workers/film-worker/film.worker.ts`, `app/src/engine-room/workers/spine-worker/spine.worker.ts` |
| the trained finder's job, on the front film's worker | `app/src/engine-room/workers/film-worker/jobs/trained-pass.ts` |
| what the stand-in's story must hold | `app/src/sample-case/story-shape.ts` |
| the step contract | `app/src/steps/step-contract.ts` |
| the tool contract | `app/src/toolbox/tool-contract.ts` |
| the film-to-screen transform | `app/src/film-viewer/film-space.ts` |
| the vertebra shapes the 3D view draws, how they are read, and how they reach the app without a fetch | `app/src/spine-3d/bones/vertebrae.json`, `app/src/spine-3d/bones/vertebrae.bin`, `app/src/spine-3d/bones/read-bones.ts`, `app/src/spine-3d/bones/load-bones.ts` |
| how a bone is shaped to a fitted vertebra | `app/src/spine-3d/placing/shape-bone.ts` |
| how a patient point lands on a film, in the app | `app/src/spine-3d/placing/film-camera.ts` |
| the 3D view's camera: no perspective to a narrow lens, and the named views | `app/src/spine-3d/looking/lens.ts`, `app/src/spine-3d/looking/named-views.ts` |
| the bone colour, the lights and the shadow of the 3D stage | `app/src/spine-3d/scene/look.ts` |
| what can be set on a drawn spine, a film and the stage | `app/src/spine-3d/scene/drawn-spine.ts`, `app/src/spine-3d/scene/film-wall.ts`, `app/src/spine-3d/scene/stage.ts` |
| the drawn spine's box and the stage's view, each worked out once per change | `app/src/spine-3d/scene/drawn-spine.test.ts`, `app/src/spine-3d/scene/stage-keeps.test.ts` |
| the 3D view as a part a screen places | `app/src/spine-3d/view/Spine3D.tsx` |
| the case edits | `app/src/case-state/case-edits.ts` |
| how the product looks, moves and speaks, and why | `DESIGN.md` |
| the design system's values and parts, never edited in the app | `app/src/look-and-feel/tokens.css`, `app/src/look-and-feel/components.css` |
| the earlier look's tokens, for parts no longer on the page | `app/src/look-and-feel/earlier-tokens.css` |
| a duration the earlier parts name that is not a token | `app/src/look-and-feel/named-timings.css` |
| the one stylesheet, which imports the shared ones in order | `app/src/look-and-feel/base.css` |
| the values of the data attributes that carry state to CSS | `app/src/look-and-feel/data-attributes.ts` |
| what a gallery page holds | `app/src/gallery/gallery-page.ts` |
| the preview notice, error wording, evidence entries, the one word for each idea | `app/src/words/preview-notice.ts`, `app/src/words/faults.ts`, `app/src/words/evidence-ledger.ts`, `app/src/words/glossary.ts` |
| the rule against outcome claims in the wording | `app/src/words/never-claim.test.ts` |

### Films and Find

| I want to change… | Open |
|---|---|
| the bar, steps, notice and the frame around the moments | `app/src/app-shell/AppFrame.tsx`, `app/src/app-shell/AppFrame.test.tsx`, `app/src/app-shell/AppFrame.gallery.tsx`, `app/src/app-shell/TopBar.tsx`, `app/src/app-shell/TopBar.gallery.tsx`, `app/src/app-shell/StepSwitch.tsx`, `app/src/app-shell/PreviewNotice.tsx` |
| the dock and its eight scenes | `app/src/guide/Dock.tsx`, `app/src/guide/Dock.css`, `app/src/guide/Dock.test.tsx`, `app/src/guide/Dock.gallery.tsx`, `app/src/guide/Dock.gallery.css`, `app/src/guide/dock-spec.ts` |
| the shared step control | `app/src/building-blocks/Segmented.tsx`, `app/src/building-blocks/Segmented.test.tsx`, `app/src/building-blocks/Segmented.gallery.tsx` |
| state carried to the design system on the page | `app/src/look-and-feel/page-state.ts`, `app/src/look-and-feel/page-state.test.tsx` |
| the rules that keep styles in the design system | `app/src/look-and-feel/names-only.test.ts` |
| the drawings in the empty film places | `app/src/look-and-feel/figures/PlaceGlyph.tsx`, `app/src/look-and-feel/figures/PlaceGlyph.gallery.tsx` |
| the stage and its persistent bench | `app/src/bench/Stage.tsx`, `app/src/bench/Stage.test.tsx`, `app/src/bench/Stage.gallery.tsx`, `app/src/bench/Stage.gallery.css`, `app/src/bench/Bench.tsx` |
| the four film places and their reading faces | `app/src/bench/FilmPlaces.tsx` |
| the film tiles and their thumbnail handover | `app/src/bench/FilmTile.tsx`, `app/src/bench/FilmTile.css`, `app/src/bench/HungFilm.tsx` |
| where films and their names sit in each moment | `app/src/bench/stage-layout.ts`, `app/src/bench/stage-layout.test.ts` |
| framing a whole film, its spine or one vertebra | `app/src/bench/film-framing.ts`, `app/src/bench/film-framing.test.ts` |
| films travelling between moments and their cameras gliding | `app/src/bench/fly-tiles.ts`, `app/src/bench/fly-tiles.test.ts`, `app/src/bench/glide-cameras.ts` |
| taking films in, choosing a place and watching a drop | `app/src/bench/film-intake.ts`, `app/src/bench/film-intake.test.ts`, `app/src/bench/assign-places.ts`, `app/src/bench/assign-places.test.ts`, `app/src/bench/drop-files.ts`, `app/src/bench/drop-files.test.tsx` |
| forgetting the films and starting over | `app/src/bench/start-over.ts`, `app/src/bench/start-over.test.ts` |
| drawing the search result and review on the stage | `app/src/bench/find-on-stage.ts`, `app/src/bench/find-on-stage.test.tsx` |
| the Films moment and its dock states | `app/src/steps/1-films/step.ts`, `app/src/steps/1-films/FilmsScreen.tsx`, `app/src/steps/1-films/FilmsScreen.test.tsx`, `app/src/steps/1-films/films-dock.ts`, `app/src/steps/1-films/films-dock.test.ts` |
| the Find moment, search run and progress | `app/src/steps/2-find/step.ts`, `app/src/steps/2-find/FindScreen.tsx`, `app/src/steps/2-find/FindScreen.test.tsx`, `app/src/steps/2-find/find-run.ts`, `app/src/steps/2-find/find-run.test.ts`, `app/src/steps/2-find/search-progress.ts`, `app/src/steps/2-find/search-progress.test.ts` |
| the Find dock, review and search-failure words | `app/src/steps/2-find/find-dock.ts`, `app/src/steps/2-find/find-dock.test.ts` |
| the finished search as a replay, its clock and player | `app/src/auto-pass/find-replay.ts`, `app/src/auto-pass/find-replay.test.ts`, `app/src/auto-pass/find-replay-clock.ts`, `app/src/auto-pass/find-replay-clock.test.ts`, `app/src/auto-pass/ReplayPlayer.tsx`, `app/src/auto-pass/ReplayPlayer.test.tsx` |
| the marks drawn on a film and their corner order | `app/src/marks/mark-to-draw.ts`, `app/src/marks/from-film-marks.ts`, `app/src/marks/from-film-marks.test.ts`, `app/src/marks/MarksOverlay.tsx`, `app/src/marks/MarksOverlay.test.tsx`, `app/src/marks/MarksOverlay.gallery.tsx` |
| the travelling light over a film | `app/src/marks/TravellingLight.tsx`, `app/src/marks/TravellingLight.test.tsx` |
| the outline styles and their gallery surroundings | `app/src/marks/marks.css`, `app/src/marks/marks.gallery.css` |
| which outlines and levels need a look | `app/src/review/needs-a-look.ts`, `app/src/review/needs-a-look.test.ts`, `app/src/review/look-levels.ts`, `app/src/review/look-levels.test.ts` |
| the current dock, film intake and Find state | `app/src/view-state/dock-now.ts`, `app/src/view-state/dock-now.test.ts`, `app/src/view-state/film-intake.ts`, `app/src/view-state/film-intake.test.ts`, `app/src/view-state/find-state.ts`, `app/src/view-state/find-state.test.ts` |
| a newly opened film as an unconfirmed case record | `app/src/open-and-save/film-record.ts`, `app/src/open-and-save/film-record.test.ts` |
| the made-up sample case and its fixed recipe | `app/src/made-up-spines/the-sample-case.ts`, `app/src/made-up-spines/the-sample-case.test.ts` |
| optional front films kept on this machine and their build rules | `app/src/sample-films/sample-films.ts`, `app/src/sample-films/sample-films.test.ts`, `app/src/sample-films/README.md` |
| the words for film places, films, Find and outlines | `app/src/words/film-names.ts`, `app/src/words/stage.ts`, `app/src/words/films.ts`, `app/src/words/find.ts`, `app/src/words/marks.ts`, `app/src/words/system-words.test.ts` |

## The proof (`proving-ground/`)

The proof ladder: `proving-ground/README.md`.

| I want to change… | Open |
|---|---|
| a golden case file or its verdict | `proving-ground/known-answers/case-files/verdicts.json` |
| the known answer for swapping left/right names | `proving-ground/known-answers/side-swap.json` |
| the known answers the 3D view's placing is held to | `proving-ground/known-answers/spine-3d/placed-bodies.json` |
| the real engine in Node | `proving-ground/parity/contract.test.ts` |
| each fitted bone on its own outline when Find keeps only some vertebrae, and the refusal with one | `proving-ground/parity/some-vertebrae-found.test.ts` |
| the trained finder's native answer, and the browser build held to it | `proving-ground/known-answers/trained-pass/front.json`, `proving-ground/parity/trained-pass.test.ts`, `engine/trained-finder/tests/finds_the_vertebrae_on_a_made_up_film.rs` |
| the offline Films-to-Find journey and its 640-wide layout checks | `proving-ground/in-the-browser/no-network/nothing-leaves.spec.ts` |
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
