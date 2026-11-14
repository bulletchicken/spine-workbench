import { BufferAttribute, BufferGeometry, Color, DoubleSide, Group, Mesh, Points, ShaderMaterial, Vector2, Vector4 } from 'three';
import type { Level } from '@/shapes';
import type { View, CanvasSize } from '../looking/view';
import { toCanvas } from '../looking/lens';
import type { FilmPlane } from '../placing/film-plane';
import { toScene } from '../placing/scene-axes';
import type { Vec3 } from '../placing/vectors';
import { threadFoot } from './formation-mesh';
import { CAGE, FORMATION } from './look';
import type { SpineModel } from './spine-model';

export interface FormationScript {
  begins: number; fills: number; first: number; spread: readonly [number, number]; loose: number;
  lines: readonly [number, number]; fade: readonly [number, number];
  reach: number; hold: number; threadFade: number; pointFade: number; pointSettle: number; nodeSettle: number;
}
export interface FormationDrawing {
  script(level: Level, script: FormationScript): void;
  threads(planes: readonly FilmPlane[]): void;
  time(ms: number, visible: boolean): void;
  prepare(view: View, size: CanvasSize, ratio: number, walls: readonly (readonly Vec3[])[]): void;
  dispose(): void;
}

const OVER_FILM = /* glsl */`
  uniform vec2 uWall[8];
  uniform vec2 uWalls;
  float beside(vec2 p, vec2 a, vec2 b) { vec2 e = b - a; return (e.x * (p.y - a.y) - e.y * (p.x - a.x)) / max(length(e), 1e-4); }
  float within(vec2 p, vec2 a, vec2 b, vec2 c, vec2 d) {
    float turn = sign((b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x));
    return clamp(min(min(beside(p, a, b) * turn, beside(p, b, c) * turn), min(beside(p, c, d) * turn, beside(p, d, a) * turn)) + 0.5, 0.0, 1.0);
  }
  float overFilm() {
    vec2 p = gl_FragCoord.xy;
    return max(uWalls.x * within(p, uWall[0], uWall[1], uWall[2], uWall[3]), uWalls.y * within(p, uWall[4], uWall[5], uWall[6], uWall[7]));
  }`;

// A point: a small plain dot. It comes out a little large and settles; when lines reach it, it
// becomes their joint; when the bone fills in, it goes with the lines.
const POINT_VERTEX = /* glsl */`
  attribute vec3 aCentre;
  attribute vec4 aTime;      // when it comes out, when lines reach it, when it starts to go, whether it is a first point
  uniform float uT, uRatio, uDot, uFirst, uNode, uFrom, uLift, uFar, uGoFor, uPointFor, uSettleFor, uJoinFor;
  varying float vAlpha;
  varying float vSize;
  void main() {
    vec4 mv = modelViewMatrix * vec4(position, 1.0);
    float nearer = mv.z - (modelViewMatrix * vec4(aCentre, 1.0)).z;
    float since = uT - aTime.x;
    float out_ = clamp(since / uPointFor, 0.0, 1.0);
    float settled = 1.0 - pow(1.0 - clamp(since / uSettleFor, 0.0, 1.0), 3.0);
    float joined = smoothstep(0.0, uJoinFor, uT - aTime.y);
    float gone = clamp((uT - aTime.z) / uGoFor, 0.0, 1.0);
    float size = uDot * mix(1.0, uFirst, aTime.w) * mix(uFrom, 1.0, settled);
    size = mix(size, uNode, joined);
    mv.xyz *= 1.0 - uLift / length(mv.xyz);
    gl_Position = projectionMatrix * mv;
    vSize = size * uRatio;
    gl_PointSize = vSize + 2.0;
    vAlpha = out_ * (1.0 - gone) * mix(uFar, 1.0, clamp(0.62 + nearer / 26.0, 0.0, 1.0));
  }`;
const POINT_FRAGMENT = /* glsl */`
  uniform vec4 uInk, uLight;
  varying float vAlpha;
  varying float vSize;
  ${OVER_FILM}
  void main() {
    float away = length(gl_PointCoord - 0.5) * (vSize + 2.0);
    vec4 c = mix(uInk, uLight, overFilm());
    float a = c.a * vAlpha * clamp(vSize * 0.5 - away + 0.5, 0.0, 1.0);
    if (a < 0.003) discard;
    gl_FragColor = vec4(c.rgb, a);
  }`;

// A line: drawn from one end to the other, hair-thin, the same width whatever its slant. Used
// for the lines of the mesh and for the threads to the films.
const LINE_VERTEX = /* glsl */`
  attribute vec3 aEnd;       // the far end; "position" is the end it grows from
  attribute vec3 aCentre;
  attribute vec2 aCorner;    // which end of the line this corner is at (0 or 1), and which side of it (-1 or 1)
  attribute vec3 aTime;      // when it starts to draw, how long it takes, when it starts to go
  uniform float uT, uRatio, uWidth, uLift, uFar, uGoFor, uEase;
  uniform vec2 uView;
  varying float vAcross;
  varying float vAlpha;
  void main() {
    float drawn = clamp((uT - aTime.x) / aTime.y, 0.0, 1.0);
    drawn = mix(drawn, drawn * drawn * (3.0 - 2.0 * drawn), uEase);
    vec4 a = modelViewMatrix * vec4(position, 1.0), b = modelViewMatrix * vec4(aEnd, 1.0);
    float nearer = mix(a.z, b.z, aCorner.x) - (modelViewMatrix * vec4(aCentre, 1.0)).z;
    a.xyz *= 1.0 - uLift / length(a.xyz); b.xyz *= 1.0 - uLift / length(b.xyz);
    vec4 ca = projectionMatrix * a, cb = projectionMatrix * b;
    vec2 along_ = (cb.xy / cb.w - ca.xy / ca.w) * uView;
    vec2 across = normalize(vec2(-along_.y, along_.x) + vec2(1e-6, 0.0));
    vec4 c = mix(ca, mix(ca, cb, drawn), aCorner.x);
    float half_ = max(uWidth * uRatio, 1.0) * 0.5 + 1.0;
    c.xy += across * aCorner.y * half_ * 2.0 / uView * c.w;
    gl_Position = c;
    vAcross = aCorner.y * half_;
    vAlpha = step(0.0005, drawn) * (1.0 - clamp((uT - aTime.z) / uGoFor, 0.0, 1.0)) * mix(uFar, 1.0, clamp(0.62 + nearer / 26.0, 0.0, 1.0));
  }`;
const LINE_FRAGMENT = /* glsl */`
  uniform vec4 uInk, uLight;
  uniform float uRatio, uWidth;
  varying float vAcross;
  varying float vAlpha;
  ${OVER_FILM}
  void main() {
    float width = uWidth * uRatio;
    vec4 c = mix(uInk, uLight, overFilm());
    float a = c.a * vAlpha * min(width, 1.0) * clamp(max(width, 1.0) * 0.5 + 0.5 - abs(vAcross), 0.0, 1.0);
    if (a < 0.003) discard;
    gl_FragColor = vec4(c.rgb, a);
  }`;


function strip(count: number) {
  const geometry = new BufferGeometry(), corners = new Float32Array(count * 8), index = new Uint32Array(count * 6);
  for (let i = 0; i < count; i++) {
    corners.set([0, -1, 0, 1, 1, -1, 1, 1], i * 8);
    index.set([i * 4, i * 4 + 1, i * 4 + 2, i * 4 + 2, i * 4 + 1, i * 4 + 3], i * 6);
  }
  for (const [name, size] of [['position', 3], ['aEnd', 3], ['aCentre', 3], ['aTime', 3]] as const) geometry.setAttribute(name, new BufferAttribute(new Float32Array(count * 4 * size), size));
  geometry.setAttribute('aCorner', new BufferAttribute(corners, 2)); geometry.setIndex(new BufferAttribute(index, 1));
  return geometry;
}
function writeLine(geometry: BufferGeometry, i: number, from: Vec3, to: Vec3, centre: Vec3, times: Vec3) {
  for (let c = 0; c < 4; c++) for (const [name, value] of [['position', from], ['aEnd', to], ['aCentre', centre], ['aTime', times]] as const) {
    const attribute = geometry.getAttribute(name); attribute.setXYZ(i * 4 + c, ...value); attribute.needsUpdate = true;
  }
}
function scatter(a: number, b: number) {
  let h = (a * 374761393 + b * 668265263) >>> 0; h = Math.imul(h ^ (h >>> 13), 1274126177) >>> 0;
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** All bones' points, all edges, and all film threads: three fixed drawings. */
export function formationDrawing(model: SpineModel, root: Group, changed: () => void): FormationDrawing {
  const time = { value: 0 }, ratio = { value: 1 }, view = { value: new Vector2(1, 1) };
  const walls = { uWall: { value: Array.from({ length: 8 }, () => new Vector2()) }, uWalls: { value: new Vector2() } };
  const colour = (name: string, strength: number) => { const c = new Color(name).convertLinearToSRGB(); return { value: new Vector4(c.r, c.g, c.b, strength) }; };
  const common = { uT: time, uRatio: ratio, ...walls, uLift: { value: FORMATION.lift }, uGoFor: { value: 1 } };
  const lineMaterial = (thread: boolean) => new ShaderMaterial({ vertexShader: LINE_VERTEX, fragmentShader: LINE_FRAGMENT, transparent: true, depthWrite: false, depthTest: false, side: DoubleSide,
    uniforms: { ...common, uView: view, uWidth: { value: FORMATION.line }, uFar: { value: thread ? 1 : FORMATION.far }, uEase: { value: thread ? 1 : 0 }, uInk: colour(CAGE.onPage, thread ? FORMATION.threadStrength : CAGE.strength), uLight: colour(CAGE.onFilm, thread ? FORMATION.lightThread : FORMATION.lightLine) } });
  const dotGeometry = new BufferGeometry();
  const dotCount = model.bones.reduce((sum, bone) => sum + bone.formation.points.length, 0);
  for (const [name, size] of [['position', 3], ['aCentre', 3], ['aTime', 4]] as const) dotGeometry.setAttribute(name, new BufferAttribute(new Float32Array(dotCount * size), size));
  const dots = new Points(dotGeometry, new ShaderMaterial({ vertexShader: POINT_VERTEX, fragmentShader: POINT_FRAGMENT, transparent: true, depthWrite: false, depthTest: false,
    uniforms: { ...common, uPointFor: { value: 1 }, uSettleFor: { value: 1 }, uJoinFor: { value: 1 }, uDot: { value: FORMATION.dot }, uFirst: { value: FORMATION.first }, uNode: { value: FORMATION.node }, uFrom: { value: FORMATION.from }, uFar: { value: FORMATION.far }, uInk: colour(FORMATION.ink, FORMATION.dotStrength), uLight: colour(CAGE.onFilm, FORMATION.lightDot) } }));
  const lines = new Mesh(strip(model.bones.reduce((sum, bone) => sum + bone.formation.edges.length, 0)), lineMaterial(false));
  const threads = new Mesh(strip(model.bones.length * 4), lineMaterial(true));
  const group = new Group(); group.visible = false; root.add(group); group.add(dots, lines, threads);
  for (const part of [dots, lines, threads]) { part.frustumCulled = false; part.renderOrder = 3; }
  dots.renderOrder = 4; threads.geometry.setDrawRange(0, 0);
  let dotAt = 0, lineAt = 0;
  const ranges = model.bones.map((bone) => {
    const at = { bone, dot: dotAt, line: lineAt }; dotAt += bone.formation.points.length; lineAt += bone.formation.edges.length;
    const centre = bone.placing.centre;
    bone.formation.points.forEach((point, i) => {
      dotGeometry.getAttribute('position').setXYZ(at.dot + i, ...bone.placing.toPatient(point));
      dotGeometry.getAttribute('aCentre').setXYZ(at.dot + i, ...centre);
    });
    return at;
  });
  const scripts = new Map<Level, FormationScript>();
  let planes: readonly FilmPlane[] = [];
  const writeThreads = () => {
    let i = 0;
    for (const { bone } of ranges) {
      const script = scripts.get(bone.level); if (script === undefined) continue;
      for (const pole of bone.formation.poles) {
        const point = bone.placing.toPatient(pole);
        for (const plane of planes) writeLine(threads.geometry, i++, threadFoot(point, plane), point, bone.placing.centre, [script.begins, script.reach, script.begins + script.hold]);
      }
      const fade = threads.material.uniforms.uGoFor; if (fade !== undefined) fade.value = script.threadFade;
    }
    threads.geometry.setDrawRange(0, i * 6);
  };
  return {
    script(level, script) {
      scripts.set(level, script);
      const found = ranges.find(({ bone }) => bone.level === level); if (found === undefined) return;
      const { bone, dot, line } = found;
      for (const [key, value] of [['uPointFor', script.pointFade], ['uSettleFor', script.pointSettle], ['uJoinFor', script.nodeSettle]] as const) { const uniform = dots.material.uniforms[key]; if (uniform !== undefined) uniform.value = value; }
      const goes = script.fills + script.fade[0];
      for (const material of [dots.material, lines.material]) { const fade = material.uniforms.uGoFor; if (fade !== undefined) fade.value = script.fade[1]; }
      bone.formation.points.forEach((_, i) => {
        const rank = bone.formation.rank[i] ?? 0, first = i < 2;
        const loose = (scatter(dot + i, line) * 2 - 1) * script.loose * Math.min(1, rank / .2);
        const appears = first ? script.first : script.spread[0] + (script.spread[1] - script.spread[0]) * rank + loose;
        dotGeometry.getAttribute('aTime').setXYZW(dot + i, script.begins + appears, script.begins + script.lines[0] + rank * script.lines[1], goes, first ? 1 : 0);
      });
      dotGeometry.getAttribute('aTime').needsUpdate = true; changed();
      bone.formation.edges.forEach(([a, b], i) => {
        const near = bone.formation.rank[a] ?? 0, far = bone.formation.rank[b] ?? 1;
        const duration = Math.max(.05, far - near);
        const start = Math.min(1 - duration, near);
        writeLine(lines.geometry, line + i, bone.placing.toPatient(bone.formation.points[a] ?? [0, 0, 0]), bone.placing.toPatient(bone.formation.points[b] ?? [0, 0, 0]), bone.placing.centre, [script.begins + script.lines[0] + start * script.lines[1], duration * script.lines[1], goes]);
      });
    },
    threads(next) { planes = next; writeThreads(); changed(); },
    time(ms, visible) { time.value = ms; group.visible = visible; changed(); },
    prepare(camera, size, deviceRatio, wallCorners) {
      ratio.value = deviceRatio; view.value.set(size.width * deviceRatio, size.height * deviceRatio);
      const dot = dots.material.uniforms.uDot; if (dot !== undefined) dot.value = FORMATION.dot * Math.max(FORMATION.leastDotScale, Math.min(1, camera.pxPerMm[0] / FORMATION.dotAtScale));
      walls.uWalls.value.set(wallCorners[0]?.length === 4 ? 1 : 0, wallCorners[1]?.length === 4 ? 1 : 0);
      wallCorners.slice(0, 2).forEach((corners, film) => { corners.forEach((point, i) => {
        const at = toCanvas(camera, size, toScene(point)); walls.uWall.value[film * 4 + i]?.set(at.x * deviceRatio, (size.height - at.y) * deviceRatio);
      }); });
    },
    dispose() { group.removeFromParent(); for (const part of [dots, lines, threads]) { part.geometry.dispose(); part.material.dispose(); } },
  };
}
