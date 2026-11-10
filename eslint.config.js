// ESLint for every TypeScript file in the repository.
//
// Besides typescript-eslint's strict rules and the React hooks rules, it
// holds one of the app's three guards against network calls: nothing in
// app/src may reach the network, navigate away or run code built from
// text, except that the one file that loads the engine's own .wasm may
// call fetch. app/src/engine-room/load-engine.test.ts fails if a
// second file is ever exempted, or if any file's rules are weakened.
// Code can always hide a name from a linter, so this guard is a backstop
// that catches the plain and the evident ways out; the content policy
// and the browser test are what hold.
// It also keeps the import boundaries between the areas of app/src, drawn
// in app/README.md. Inline disable comments are ignored everywhere.

import js from '@eslint/js';
import reactHooks from 'eslint-plugin-react-hooks';
import { defineConfig } from 'eslint/config';
import tseslint from 'typescript-eslint';

const NETWORK = 'The app makes no network calls: everything stays on this machine.';

/** Global names that reach the network. `open` loads a page in a new window. */
const NETWORK_GLOBALS = [
  'fetch',
  'XMLHttpRequest',
  'WebSocket',
  'EventSource',
  'WebTransport',
  'RTCPeerConnection',
  'Request',
  'importScripts',
  'open',
];

/**
 * The same names, plus navigator's, wherever they are reached as a
 * property, taken out by destructuring, or written as a string. `open` is
 * left out here, since many harmless objects have one.
 */
const NETWORK_NAMES = `^(${[...NETWORK_GLOBALS.filter((name) => name !== 'open'), 'sendBeacon', 'serviceWorker'].join('|')})$`;

/** The objects the global network names hang from. */
const GLOBAL_OBJECTS = '^(globalThis|self|window|navigator)$';

const NAVIGATION = 'The app never navigates away: the content policy cannot stop a page loading from anywhere.';

/** The only files allowed to call `fetch`. Exactly one, ever. */
const NETWORK_EXEMPT = ['app/src/engine-room/load-engine.ts'];

const restrictedGlobals = (names) => ['error', ...names.map((name) => ({ name, message: NETWORK }))];

// Import boundaries between the areas of app/src, drawn in app/README.md
// (which also says how a new area gets its row). Each area lists what it
// may not import. A later row replaces an earlier one for the files it
// matches, so a file gets the rules of the most specific area it is in,
// and each row carries its whole list.
const SHAPES_ONLY = { group: ['**/shapes', '**/shapes/**', '!@/shapes'], message: 'Shared types come only from @/shapes.' };
const ENGINE_INSIDES = {
  group: ['**/wasm-build', '**/wasm-build/**', '**/workers', '**/workers/**'],
  message: 'Only the engine room reaches the wasm build and the workers.',
};
const STAND_IN = { group: ['**/stand-in', '**/stand-in/**'], message: 'Only pick-engine.ts and tests use the stand-in engine.' };
const SAMPLE_CASE = {
  group: ['**/sample-case', '**/sample-case/**'],
  message: 'The sample case is for gallery pages, scenes, tests and the stand-in.',
};
const OTHER_STEPS = { group: ['**/[0-9]-*', '**/[0-9]-*/**'], message: 'A step never imports another step.' };
// The stand-in is left to STAND_IN, so that tests in these areas may use it.
const ENGINE_DOOR_ONLY = {
  group: [
    '**/engine-room/**',
    '!@/engine-room/engine-client',
    '!@/engine-room/use-engine',
    '!@/engine-room/stand-in',
    '!@/engine-room/stand-in/**',
  ],
  message: 'This area reaches the engine only through @/engine-room/engine-client and @/engine-room/use-engine.',
};

/**
 * An area's folder: its @/ path, or anything inside it by any path. The
 * bare name is matched only as @/<area>, because words files share their
 * area's name (@/words/auto-pass is not the auto-pass area).
 */
const no = (area) => ({ group: [`@/${area}`, `**/${area}/**`], message: `This area never imports ${area}.` });
const NO_STEPS = no('steps');
const NO_CASE_STATE = no('case-state');
const NO_VIEW_STATE = no('view-state');
const NO_ENGINE_ROOM = no('engine-room');
const NO_BENCH = no('bench');
const NO_AUTO_PASS = no('auto-pass');
const NO_TOOLBOX = no('toolbox');

/** Every area that look-and-feel, choreography and building-blocks never import. */
const ABOVE_THE_BLOCKS = [
  'steps',
  'case-state',
  'view-state',
  'engine-room',
  'film-viewer',
  'bench',
  'marks',
  'ladder',
  'guide',
  'auto-pass',
  'toolbox',
  'review',
  'on-this-device',
  'case-sheet',
  'reopen-case',
  'made-up-spines',
  'open-and-save',
  'app-shell',
  'gallery',
];
// Their own gallery pages need the page type. The bare @/gallery is left
// out, because a folder that matches can never be let back in.
const GALLERY_PAGE_TYPE_ONLY = {
  group: ['**/gallery/**', '!@/gallery/gallery-page'],
  message: 'This area takes only the GalleryPage type from @/gallery/gallery-page.',
};
const DRAWS_ONLY = ABOVE_THE_BLOCKS.map((area) => (area === 'gallery' ? GALLERY_PAGE_TYPE_ONLY : no(area)));
const REVIEW_ONLY = [
  ...ABOVE_THE_BLOCKS.filter((area) => area !== 'review').map(no),
  no('choreography'),
  no('building-blocks'),
  { group: ['**/look-and-feel/**', '!@/look-and-feel/data-attributes'], message: 'review/ takes only the data-attribute types from look-and-feel.' },
];

const AREAS = [
  { files: 'app/src/**/*.{ts,tsx}', banned: [SHAPES_ONLY, ENGINE_INSIDES] },
  { files: 'app/src/shapes/**/*.{ts,tsx}', banned: [ENGINE_INSIDES] },
  { files: 'app/src/engine-room/**/*.{ts,tsx}', banned: [SHAPES_ONLY] },
  { files: 'app/src/steps/[0-9]-*/**/*.{ts,tsx}', banned: [SHAPES_ONLY, ENGINE_INSIDES, OTHER_STEPS, ENGINE_DOOR_ONLY] },
  { files: 'app/src/{look-and-feel,choreography,building-blocks}/**/*.{ts,tsx}', banned: [SHAPES_ONLY, ENGINE_INSIDES, ...DRAWS_ONLY] },
  {
    files: 'app/src/film-viewer/**/*.{ts,tsx}',
    banned: [SHAPES_ONLY, ENGINE_INSIDES, NO_STEPS, NO_CASE_STATE, NO_VIEW_STATE, NO_ENGINE_ROOM],
  },
  {
    files: 'app/src/{marks,ladder,guide}/**/*.{ts,tsx}',
    banned: [SHAPES_ONLY, ENGINE_INSIDES, NO_STEPS, NO_CASE_STATE, NO_VIEW_STATE, NO_ENGINE_ROOM, NO_BENCH, NO_AUTO_PASS, NO_TOOLBOX],
  },
  { files: 'app/src/review/**/*.{ts,tsx}', banned: [SHAPES_ONLY, ENGINE_INSIDES, ...REVIEW_ONLY] },
  {
    files: 'app/src/auto-pass/**/*.{ts,tsx}',
    banned: [SHAPES_ONLY, ENGINE_INSIDES, NO_STEPS, NO_CASE_STATE, NO_BENCH, NO_TOOLBOX, ENGINE_DOOR_ONLY],
  },
  {
    files: 'app/src/{bench,toolbox,on-this-device,case-sheet,reopen-case,made-up-spines,open-and-save}/**/*.{ts,tsx}',
    banned: [SHAPES_ONLY, ENGINE_INSIDES, NO_STEPS, ENGINE_DOOR_ONLY],
  },
];

/** Where the stand-in engine may be imported. */
const STAND_IN_ALLOWED = ['app/src/engine-room/pick-engine.ts', 'app/src/engine-room/stand-in/**', '**/*.test.{ts,tsx}'];

/** Where the sample case may be imported: those, and the files that show the app in a state. */
const SAMPLE_CASE_ALLOWED = [...STAND_IN_ALLOWED, 'app/src/sample-case/**', '**/*.gallery.tsx', '**/*.scenes.ts'];

// Three entries per row, in this order: every file of the area gets its
// list and both bans; files where the sample case is allowed lose that
// ban; files where the stand-in is allowed lose both.
const importBoundaries = AREAS.flatMap(({ files, banned }) => [
  { files: [files], rules: { 'no-restricted-imports': ['error', { patterns: [...banned, STAND_IN, SAMPLE_CASE] }] } },
  {
    files: SAMPLE_CASE_ALLOWED.map((allowed) => [files, allowed]),
    rules: { 'no-restricted-imports': ['error', { patterns: [...banned, STAND_IN] }] },
  },
  {
    files: STAND_IN_ALLOWED.map((allowed) => [files, allowed]),
    rules: { 'no-restricted-imports': ['error', { patterns: banned }] },
  },
]);

export default defineConfig(
  {
    // Written by tools, never edited by hand; or not ours.
    ignores: [
      '**/node_modules/',
      '**/dist/',
      'target/',
      'local-only/',
      'app/src/shapes/generated/',
      'app/src/engine-room/wasm-build/',
      'proving-ground/test-results/',
    ],
  },
  {
    // An inline disable comment does nothing and is reported; `eslint
    // --max-warnings 0` turns that report into a failure.
    linterOptions: { noInlineConfig: true },
  },
  js.configs.recommended,
  tseslint.configs.strictTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    // This file is plain JavaScript outside every TypeScript project.
    files: ['**/*.js'],
    extends: [tseslint.configs.disableTypeChecked],
  },
  {
    files: ['app/src/**/*.{ts,tsx}'],
    extends: [reactHooks.configs.flat['recommended-latest']],
  },
  {
    // The network ban.
    files: ['app/src/**/*.{ts,tsx}'],
    rules: {
      'no-restricted-globals': restrictedGlobals(NETWORK_GLOBALS),
      // Code built from text could name anything.
      'no-eval': 'error',
      'no-new-func': 'error',
      'no-restricted-syntax': [
        'error',
        { selector: `MemberExpression[property.name=/${NETWORK_NAMES}/]`, message: NETWORK },
        { selector: `Property[key.name=/${NETWORK_NAMES}/]`, message: NETWORK },
        {
          selector: `MemberExpression[computed=true][property.type!="Literal"][object.name=/${GLOBAL_OBJECTS}/]`,
          message: `${NETWORK} A global's property is named in full, never computed.`,
        },
        {
          // An alias of a global object would escape the two rules above.
          selector: `Identifier[name=/${GLOBAL_OBJECTS}/]:not(MemberExpression > Identifier.object, MemberExpression > Identifier.property, Property > Identifier.key, TSQualifiedName > Identifier, TSTypeQuery > Identifier.exprName, UnaryExpression[operator="typeof"] > Identifier.argument)`,
          message: `${NETWORK} A global object is reached through a named property, never passed around or aliased.`,
        },
        { selector: 'MemberExpression[property.name="open"][object.name=/^(globalThis|self|window)$/]', message: NAVIGATION },
        { selector: 'AssignmentExpression > Identifier.left[name="location"]', message: NAVIGATION },
        { selector: 'AssignmentExpression > MemberExpression.left[object.name="location"]', message: NAVIGATION },
        { selector: 'AssignmentExpression > MemberExpression.left[property.name="location"]', message: NAVIGATION },
        { selector: 'AssignmentExpression > MemberExpression.left[object.property.name="location"]', message: NAVIGATION },
        { selector: 'MemberExpression[property.name=/^(assign|replace)$/][object.name="location"]', message: NAVIGATION },
        { selector: 'MemberExpression[property.name=/^(assign|replace)$/][object.property.name="location"]', message: NAVIGATION },
        { selector: `Literal[value=/${NETWORK_NAMES}/]`, message: `${NETWORK} This string names a network API.` },
        { selector: `TemplateElement[value.raw=/${NETWORK_NAMES}/]`, message: `${NETWORK} This string names a network API.` },
        { selector: 'ImportExpression[source.type!="Literal"]', message: 'import() takes a relative path written out in full.' },
        { selector: 'ImportExpression > Literal[value!=/^\\.{1,2}\\//]', message: 'import() takes a relative path written out in full.' },
      ],
    },
  },
  {
    // The one exemption: the engine's own .wasm, from the page's origin.
    files: NETWORK_EXEMPT,
    rules: {
      'no-restricted-globals': restrictedGlobals(NETWORK_GLOBALS.filter((name) => name !== 'fetch')),
    },
  },
  ...importBoundaries,
);
