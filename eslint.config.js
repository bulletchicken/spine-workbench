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

// Import boundaries between the areas of app/src. Each area lists what it
// may not import; a file gets the rules of the most specific area it is in.
const SHAPES_ONLY = { group: ['**/shapes', '**/shapes/**', '!@/shapes'], message: 'Shared types come only from @/shapes.' };
const ENGINE_INSIDES = {
  group: ['**/wasm-build', '**/wasm-build/**', '**/workers', '**/workers/**'],
  message: 'Only the engine room reaches the wasm build and the workers.',
};
const STAND_IN = { group: ['**/stand-in', '**/stand-in/**'], message: 'Only pick-engine.ts and tests use the stand-in engine.' };
const OTHER_STEPS = { group: ['**/[0-9]-*', '**/[0-9]-*/**'], message: 'A step never imports another step.' };
const STEP_ENGINE = {
  group: ['**/engine-room/**', '!@/engine-room/engine-client', '!@/engine-room/use-engine'],
  message: 'Steps reach the engine only through @/engine-room/engine-client and @/engine-room/use-engine.',
};
const NO_STEPS = { group: ['**/steps', '**/steps/**'], message: 'This area never imports steps.' };
const NO_CASE_STATE = { group: ['**/case-state', '**/case-state/**'], message: 'This area never imports the case state.' };
const NO_ENGINE_ROOM = { group: ['**/engine-room', '**/engine-room/**'], message: 'This area never imports the engine room.' };

const AREAS = [
  { files: 'app/src/**/*.{ts,tsx}', banned: [SHAPES_ONLY, ENGINE_INSIDES] },
  { files: 'app/src/shapes/**/*.{ts,tsx}', banned: [ENGINE_INSIDES] },
  { files: 'app/src/engine-room/**/*.{ts,tsx}', banned: [SHAPES_ONLY] },
  { files: 'app/src/steps/[0-9]-*/**/*.{ts,tsx}', banned: [SHAPES_ONLY, ENGINE_INSIDES, OTHER_STEPS, STEP_ENGINE] },
  {
    files: 'app/src/{film-viewer,look-and-feel,choreography,building-blocks}/**/*.{ts,tsx}',
    banned: [SHAPES_ONLY, ENGINE_INSIDES, NO_STEPS, NO_CASE_STATE, NO_ENGINE_ROOM],
  },
];

/** Where the stand-in engine may be imported. */
const STAND_IN_ALLOWED = ['app/src/engine-room/pick-engine.ts', 'app/src/engine-room/stand-in/**', '**/*.test.{ts,tsx}'];

const importBoundaries = AREAS.flatMap(({ files, banned }) => [
  { files: [files], rules: { 'no-restricted-imports': ['error', { patterns: [...banned, STAND_IN] }] } },
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
