/*
 * What a wasm failure is, shared by the worker that opens a file (worker.ts)
 * and the one that reads an element's saved parameters (parameters-worker.ts).
 *
 * The panic hook the bindings install writes `rvt-rs wasm panic: <info>` to
 * `console.error` before the trap. A trap with none of those first is not a
 * panic: an allocation the wasm memory could not grow to hold aborts the
 * instance without running the hook, and reaches here as a bare
 * `RuntimeError: unreachable` (V8), `unreachable executed` (Firefox) or
 * `Unreachable code should not be executed` (Safari).
 */

export const PANIC_PREFIX = 'rvt-rs wasm panic:';

/** Panic messages and engine errors that mean an allocation could not be made. */
export const OUT_OF_MEMORY_MESSAGE =
  /capacity overflow|memory allocation of \d+ bytes failed|out of memory|cannot allocate|array buffer allocation failed|invalid (typed )?array length|maximum memory size exceeded|invalid string length/i;

/**
 * True when `err` is the wasm instance, or the engine around it, running out
 * of memory. wasm32 holds at most 4 GiB, a `Vec` at most 2 GiB, and a failed
 * allocation aborts without a message, so the evidence is: a bare
 * `unreachable` trap with no panic message before it, or a message that names
 * an allocation. Any other panic is a bug and keeps its message.
 */
export function outOfMemory(err: unknown, panic: string | null): boolean {
  const message = err instanceof Error ? err.message : String(err);
  if (panic !== null) return OUT_OF_MEMORY_MESSAGE.test(panic);
  if (err instanceof WebAssembly.RuntimeError) {
    return /unreach/i.test(message) || OUT_OF_MEMORY_MESSAGE.test(message);
  }
  return OUT_OF_MEMORY_MESSAGE.test(message) && !(err instanceof TypeError);
}
