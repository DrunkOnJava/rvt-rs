/** The viewer's package version, set by `vite.config.ts`. */
declare const __RVT_VIEWER_VERSION__: string;

/**
 * A worker bundled into the page and started from a blob URL (`?worker&inline`),
 * so that starting it fetches nothing.
 */
declare module '*?worker&inline' {
  const WorkerWrapper: new (options?: { name?: string }) => Worker;
  export default WorkerWrapper;
}
