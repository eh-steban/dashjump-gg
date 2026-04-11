/**
 * Minimap coordinate utilities.
 *
 * These helpers operate on the generic shape `{ x, y, ... }` so they can be
 * reused across any entity type that carries world coordinates (bosses,
 * sinners, etc.) without coupling the helper to a specific domain model.
 */

/** A function that projects world-space (x, y) onto the minimap in pixel space. */
export type WorldToMinimapPixels = (
  x: number,
  y: number
) => { left: number; top: number };

/**
 * Projects each snapshot's world coordinates onto the minimap, returning a new
 * array whose entries carry both the original fields and the derived
 * `left` / `top` pixel positions.
 *
 * Generic over any snapshot shape that has numeric `x` and `y`. Other fields
 * are preserved exactly. If `x` or `y` are ever renamed on a snapshot type,
 * TypeScript will fail at the call site and the mapping won't silently emit
 * 0, 0 icons.
 */
export function scaleSnapshots<T extends { x: number; y: number }>(
  snapshots: T[],
  worldToMinimapPixels: WorldToMinimapPixels
): (T & { left: number; top: number })[] {
  return snapshots.map((snapshot) => ({
    ...snapshot,
    ...worldToMinimapPixels(snapshot.x, snapshot.y),
  }));
}
