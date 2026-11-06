/*
 * An element's saved parameters (#35, RE-207) as the info panel shows them.
 *
 * The data is what `rvt-elements` prints as an element's `parameters`, read by
 * the `savedParameters` binding (rvt-wasm/src/lib.rs, docs/viewer-api.md); this
 * file only says how a value reads. Nothing is named, converted or guessed
 * beyond what the entry carries: a name is the definition's caption or absent,
 * a unit is the one the reader derived from the definition's spec, and a value
 * is as saved, in Revit's internal units.
 */

/** Where a parameter is defined (`ParameterKind` in the reader). */
export type ParameterKind =
  | 'built_in'
  | 'family'
  | 'shared'
  | 'global'
  | 'project'
  | 'undefined';

/** One saved parameter, as `rvt-elements` prints it. */
export interface SavedParameter {
  /** The definition's caption; null for a built-in or an id with no definition. */
  name: string | null;
  parameter_id: number;
  kind: ParameterKind;
  /** `String`, `Integer`, `Double` or `ElementId`. */
  storage_type: string;
  spec_type_id: string | null;
  group_type_id: string | null;
  /** As saved, in internal units; a Yes/No value is a boolean. */
  value: unknown;
  si_value: number | null;
  si_unit: string | null;
  /** The material or family type an ElementId value names. */
  target_name: string | null;
  target_family: string | null;
  /** Whether the value is the element's own or its type's. */
  level: 'instance' | 'type';
  source_element_id: number;
  source_class: string;
}

/** What the `savedParameters` binding returns (rvt-wasm/src/lib.rs, docs/viewer-api.md). */
export interface SavedParametersResult {
  release: number;
  /** False for a release whose saved parameters are not read. */
  release_read: boolean;
  /** True for a release read only experimentally: read, but outside `releases_measured`. */
  release_experimental: boolean;
  /** The first and last release whose saved parameters are read. */
  releases_read: [number, number];
  /** The first and last release the reader is measured on; those read before it are experimental. */
  releases_measured: [number, number];
  /** By ElementId; an element with no native record is not listed. */
  elements: Record<string, SavedParameter[] | undefined>;
  unresolved_family_slots: number;
  instance_parameters_without_value: number;
}

/** A read, with what it cost (the worker's report). */
export interface SavedParametersReading extends SavedParametersResult {
  readMs: number;
  wasmMemoryBytes: number;
}

/** The unit Revit saves a value in, for the SI unit the reader derived from its spec. */
const INTERNAL_UNIT: Record<string, string> = { m: 'ft', m2: 'ft²', m3: 'ft³', rad: 'rad' };
/** How the SI unit is written. */
const SI_UNIT_LABEL: Record<string, string> = { m: 'm', m2: 'm²', m3: 'm³', rad: 'rad' };

/** How a definition's owner reads, for the row. */
const KIND_TEXT: Record<string, string> = {
  built_in: 'Built-in parameter',
  family: 'Family parameter',
  shared: 'Shared parameter',
  global: 'Global parameter',
  project: 'Project parameter',
  undefined: 'No definition in the file',
};

/** The row's second line: where the parameter is defined. */
export function parameterKindText(p: SavedParameter): string {
  return KIND_TEXT[p.kind] ?? p.kind;
}

/** What the releases a file may be of are called, in words: "2015 to 2027 (2015 to 2022 experimentally)". */
export function releasesText(reading: SavedParametersResult): string {
  const [first, last] = reading.releases_read;
  const [measuredFirst] = reading.releases_measured;
  return measuredFirst > first
    ? `Revit ${first} to ${last} (${first} to ${measuredFirst - 1} experimentally)`
    : `Revit ${first} to ${last}`;
}

export const BUILT_IN_WHY =
  "A Revit built-in parameter. The file stores its id and value but no name, and rvt-rs does not ship Revit's catalog of built-in names (#255).";
export const UNDEFINED_WHY =
  'The file holds no definition for this id, so it has no name here.';

function formatNumber(n: number): string {
  return n.toLocaleString('en-US', { maximumFractionDigits: 6, useGrouping: false });
}

/** The row's key: the definition's caption, or the id of a parameter that has none. */
export function parameterLabel(p: SavedParameter): { label: string; why: string | null } {
  if (p.name !== null && p.name !== '') return { label: p.name, why: null };
  if (p.kind === 'built_in') return { label: `Built-in ${p.parameter_id}`, why: BUILT_IN_WHY };
  return { label: `Parameter ${p.parameter_id}`, why: UNDEFINED_WHY };
}

/** The row's value: as saved, with its unit where the reader knows it. */
export function parameterValueText(p: SavedParameter): string {
  const v = p.value;
  if (typeof v === 'boolean') return v ? 'Yes' : 'No';
  if (typeof v === 'string') return v === '' ? 'Empty' : v;
  if (typeof v === 'number') {
    if (p.storage_type === 'ElementId') {
      const target =
        p.target_name === null
          ? null
          : p.target_family === null
            ? p.target_name
            : `${p.target_family}: ${p.target_name}`;
      return target === null ? `ElementId ${v}` : `${target} · ElementId ${v}`;
    }
    const internal = p.si_unit === null ? undefined : INTERNAL_UNIT[p.si_unit];
    if (internal === undefined || p.si_value === null) return formatNumber(v);
    if (p.si_unit === 'rad') return `${formatNumber(v)} rad`;
    return `${formatNumber(v)} ${internal} · ${formatNumber(p.si_value)} ${SI_UNIT_LABEL[p.si_unit!] ?? p.si_unit}`;
  }
  return JSON.stringify(v);
}

/** Whether the value is a number the row aligns as a measurement. */
export function parameterIsNumeric(p: SavedParameter): boolean {
  return typeof p.value === 'number' && p.storage_type !== 'ElementId';
}

/** An element's ElementId from the info panel's `type_guid`, which the export writes as the IFC Tag. */
export function elementIdOf(tag: string | null | undefined): number | null {
  if (tag === null || tag === undefined || !/^\d+$/.test(tag)) return null;
  const id = Number(tag);
  return Number.isSafeInteger(id) ? id : null;
}
