import { describe, expect, it } from 'vitest';
import { scaleSnapshots } from '../../src/utils/minimap';

// A stand-in for the real worldToMinimapPixels. Returns deterministic pixel
// coordinates derived from the input so each assertion can verify the exact
// mapping was applied.
const stubProject = (x: number, y: number) => ({ left: x * 2, top: y * 3 });

describe('scaleSnapshots', () => {
  it('returns an empty array when given no snapshots', () => {
    const result = scaleSnapshots([], stubProject);
    expect(result).toEqual([]);
  });

  it('projects x and y onto left and top via the provided function', () => {
    const snapshots = [{ x: 10, y: 20 }];

    const result = scaleSnapshots(snapshots, stubProject);

    expect(result).toEqual([{ x: 10, y: 20, left: 20, top: 60 }]);
  });

  it('preserves every non-coordinate field on the snapshot', () => {
    type Snap = {
      x: number;
      y: number;
      id: number;
      label: string;
      meta: { nested: boolean };
    };
    const snapshots: Snap[] = [
      { x: 1, y: 2, id: 99, label: 'sinner-A', meta: { nested: true } },
    ];

    const [scaled] = scaleSnapshots(snapshots, stubProject);

    expect(scaled.id).toBe(99);
    expect(scaled.label).toBe('sinner-A');
    expect(scaled.meta).toEqual({ nested: true });
    expect(scaled.left).toBe(2);
    expect(scaled.top).toBe(6);
  });

  it('maps every snapshot independently', () => {
    const snapshots = [
      { x: 0, y: 0, id: 1 },
      { x: 5, y: 5, id: 2 },
      { x: -4, y: 7, id: 3 },
    ];

    const result = scaleSnapshots(snapshots, stubProject);

    expect(result).toEqual([
      { x: 0, y: 0, id: 1, left: 0, top: 0 },
      { x: 5, y: 5, id: 2, left: 10, top: 15 },
      { x: -4, y: 7, id: 3, left: -8, top: 21 },
    ]);
  });

  it('does not mutate the input array', () => {
    const snapshots = [{ x: 1, y: 2, id: 1 }];
    const snapshotsRef = snapshots;

    scaleSnapshots(snapshots, stubProject);

    expect(snapshots).toBe(snapshotsRef);
    expect(snapshots[0]).toEqual({ x: 1, y: 2, id: 1 });
  });

  it('calls the projection function once per snapshot with the right arguments', () => {
    const calls: Array<[number, number]> = [];
    const spyProject = (x: number, y: number) => {
      calls.push([x, y]);
      return { left: 0, top: 0 };
    };

    scaleSnapshots(
      [
        { x: 1, y: 2 },
        { x: 3, y: 4 },
      ],
      spyProject
    );

    expect(calls).toEqual([
      [1, 2],
      [3, 4],
    ]);
  });
});
