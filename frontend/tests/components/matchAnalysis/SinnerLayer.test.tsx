import { describe, expect, it } from 'vitest';
import { render, cleanup } from 'vitest-browser-react';
import { page } from 'vitest/browser';
import SinnerLayer, { ICON_SIZE } from '../../../src/components/matchAnalysis/SinnerLayer';
import { ScaledSinnerSnapshot } from '../../../src/domain/sinner';

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

function makeSnapshot(
  overrides: Partial<ScaledSinnerSnapshot> = {}
): ScaledSinnerSnapshot {
  return {
    entity_index: 1,
    x: 0,
    y: 0,
    z: 0,
    spawn_time_s: 10,
    max_health: 1000,
    death_time_s: 60,
    time_alive_s: 50,
    killer_player_slot: null,
    retaliation_damage: {},
    left: 100,
    top: 200,
    ...overrides,
  };
}

function sinnerIcons() {
  return page.getByRole('img', { name: 'Sinner' }).elements();
}

// --------------------------------------------------------------------------
// SinnerLayer -- alive filter
// --------------------------------------------------------------------------

describe('SinnerLayer', () => {
  it('renders an icon when currentSec is between spawn_time_s and death_time_s', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 10, death_time_s: 60 });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={30} />
    );

    expect(sinnerIcons().length).toBe(1);
  });

  it('renders an icon at currentSec === spawn_time_s (inclusive lower bound)', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 10, death_time_s: 60 });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={10} />
    );

    expect(sinnerIcons().length).toBe(1);
  });

  it('does not render an icon before spawn_time_s', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 10, death_time_s: 60 });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={5} />
    );

    expect(sinnerIcons().length).toBe(0);
  });

  it('does not render an icon at death_time_s (exclusive upper bound)', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 10, death_time_s: 60 });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={60} />
    );

    expect(sinnerIcons().length).toBe(0);
  });

  it('does not render an icon after death_time_s', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 10, death_time_s: 60 });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={90} />
    );

    expect(sinnerIcons().length).toBe(0);
  });

  it('keeps rendering indefinitely when death_time_s is null', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 10, death_time_s: null });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={9999} />
    );

    expect(sinnerIcons().length).toBe(1);
  });

  it('does not render a null-death snapshot before its spawn_time_s', async () => {
    const snapshot = makeSnapshot({ spawn_time_s: 100, death_time_s: null });

    render(
      <SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={50} />
    );

    expect(sinnerIcons().length).toBe(0);
  });

  it('handles recycled entity_index -- only alive snapshot renders in each window', async () => {
    // First lifecycle: entity 5 spawns at 10, dies at 60
    const first = makeSnapshot({
      entity_index: 5,
      spawn_time_s: 10,
      death_time_s: 60,
      left: 100,
      top: 100,
    });
    // Second lifecycle: entity 5 respawns at 120
    const second = makeSnapshot({
      entity_index: 5,
      spawn_time_s: 120,
      death_time_s: null,
      left: 200,
      top: 200,
    });

    // At sec 30: only first is alive
    render(
      <SinnerLayer
        scaledSinnerSnapshots={[first, second]}
        currentSec={30}
      />
    );
    expect(sinnerIcons().length).toBe(1);
    cleanup();

    // At sec 70: both are dead/not-yet-spawned -- no icons
    render(
      <SinnerLayer
        scaledSinnerSnapshots={[first, second]}
        currentSec={70}
      />
    );
    expect(sinnerIcons().length).toBe(0);
  });

  it('renders nothing when snapshot list is empty', async () => {
    render(<SinnerLayer scaledSinnerSnapshots={[]} currentSec={100} />);

    expect(sinnerIcons().length).toBe(0);
  });

  it('positions the icon centered at (left, top)', async () => {
    const snapshot = makeSnapshot({
      spawn_time_s: 0,
      death_time_s: null,
      left: 200,
      top: 300,
    });

    render(<SinnerLayer scaledSinnerSnapshots={[snapshot]} currentSec={5} />);

    const [icon] = sinnerIcons() as HTMLImageElement[];
    expect(icon).toBeDefined();
    // Centering math: top-left corner is offset by half the icon size
    expect(icon.style.left).toBe(`${200 - ICON_SIZE / 2}px`);
    expect(icon.style.top).toBe(`${300 - ICON_SIZE / 2}px`);
    expect(icon.style.width).toBe(`${ICON_SIZE}px`);
    expect(icon.style.height).toBe(`${ICON_SIZE}px`);
  });
});
