import midBossIconUrl from '../../assets/mid-boss-icon.png';
import { MidBossData } from '../../domain/midBoss';
import { findActiveCycle } from '../../services/midBoss/selectors';
import { WorldToMinimapPixels } from '../../utils/minimap';

export const ICON_SIZE = 36; // px -- slightly bigger than the sinner icon; the mid-boss is a feature event.

// Mid-boss spawns at the centre of the map (the sewers pit). The parser
// reports entity_position on each kill event, but the pit itself never
// moves -- hardcoding world (0, 0) avoids a blank icon for the window
// between spawn and first kill and for matches where the boss is never
// killed.
const PIT_WORLD_X = 0;
const PIT_WORLD_Y = 0;

interface MidBossLayerProps {
  midBoss: MidBossData;
  currentSec: number;
  worldToMinimapPixels: WorldToMinimapPixels;
}

const MidBossLayer = ({
  midBoss,
  currentSec,
  worldToMinimapPixels,
}: MidBossLayerProps) => {
  const cycle = findActiveCycle(midBoss, currentSec);
  if (!cycle) return null;

  const { left, top } = worldToMinimapPixels(PIT_WORLD_X, PIT_WORLD_Y);

  return (
    <img
      src={midBossIconUrl}
      alt='Mid-Boss'
      className='pointer-events-none absolute'
      style={{
        left: left - ICON_SIZE / 2,
        top: top - ICON_SIZE / 2,
        width: ICON_SIZE,
        height: ICON_SIZE,
      }}
    />
  );
};

export default MidBossLayer;
