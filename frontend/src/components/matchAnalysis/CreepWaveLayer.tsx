import { LaneCreepData, WaveMeta } from "../../domain/creep";
import CreepWaveIndicator from "./CreepWaveIndicator";

interface CreepWaveLayerProps {
  laneCreepData: LaneCreepData;
  currentSec: number;
  worldToMinimapPixels: (x: number, y: number) => { left: number; top: number };
}

// Returns true when a wave pin should be shown at the given second.
// A pin is shown when the wave has died (last_death_sec is set and passed) AND
// no newer wave on the same lane+team has also died by that second.
function isPinVisible(
  waveId: string,
  meta: WaveMeta,
  currentSec: number,
  allMeta: Record<string, WaveMeta>
): boolean {
  if (meta.last_death_sec === null) return false;
  if (currentSec < meta.last_death_sec) return false;

  // Check if a newer wave (same lane+team, later spawn_sec) has died by now
  for (const [otherId, other] of Object.entries(allMeta)) {
    if (otherId === waveId) continue;
    if (other.lane !== meta.lane || other.team !== meta.team) continue;
    if (other.spawn_sec <= meta.spawn_sec) continue;
    if (other.last_death_sec !== null && other.last_death_sec <= currentSec) {
      return false;
    }
  }

  return true;
}

const CreepWaveLayer = ({
  laneCreepData,
  currentSec,
  worldToMinimapPixels,
}: CreepWaveLayerProps) => {
  const indicators: React.ReactElement[] = [];

  // Pass 1 — Live creeps: render individual dots at their current position
  for (const [entityIndex, timeline] of Object.entries(laneCreepData.creeps)) {
    const snapshot =
      currentSec < timeline.length ? timeline[currentSec] : null;
    if (!snapshot) continue;

    const { left, top } = worldToMinimapPixels(snapshot.x, snapshot.y);

    indicators.push(
      <CreepWaveIndicator
        key={`creep-${entityIndex}`}
        left={left}
        top={top}
        mode="live"
        team={snapshot.team}
      />
    );
  }

  // Pass 2 — Pins: render last-death position for the most-recently-dead wave per lane+team
  for (const [waveId, meta] of Object.entries(laneCreepData.wave_meta)) {
    if (!isPinVisible(waveId, meta, currentSec, laneCreepData.wave_meta)) {
      continue;
    }

    if (meta.last_death_x === null || meta.last_death_y === null) continue;

    const { left, top } = worldToMinimapPixels(
      meta.last_death_x,
      meta.last_death_y
    );

    indicators.push(
      <CreepWaveIndicator
        key={`pin-${waveId}`}
        left={left}
        top={top}
        mode="pin"
        team={meta.team}
      />
    );
  }

  return <>{indicators}</>;
};

export { isPinVisible };
export default CreepWaveLayer;
