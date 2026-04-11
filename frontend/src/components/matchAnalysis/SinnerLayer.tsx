import sinnerIconUrl from '../../assets/map-sacrifice-machine-icon.png';
import { ScaledSinnerSnapshot } from '../../domain/sinner';

interface SinnerLayerProps {
  scaledSinnerSnapshots: ScaledSinnerSnapshot[];
  currentSec: number;
}

export const ICON_SIZE = 28; // px -- adjust if the icon looks too large or small on the 768px minimap

const SinnerLayer = ({ scaledSinnerSnapshots, currentSec }: SinnerLayerProps) => {
  const alive = scaledSinnerSnapshots.filter(
    (s) =>
      currentSec >= s.spawn_time_s &&
      (s.death_time_s === null || currentSec < s.death_time_s)
  );

  return (
    <>
      {alive.map((s) => (
        <img
          key={`sinner-${s.entity_index}-${s.spawn_time_s}`}
          src={sinnerIconUrl}
          alt='Sinner'
          className='pointer-events-none absolute'
          style={{
            left: s.left - ICON_SIZE / 2,
            top: s.top - ICON_SIZE / 2,
            width: ICON_SIZE,
            height: ICON_SIZE,
          }}
        />
      ))}
    </>
  );
};

export default SinnerLayer;
