import { useRef, useState, useEffect, Dispatch, SetStateAction } from 'react';
import Grid from './Grid';
import Objectives from './Objectives';
import RegionToggle from './RegionToggle';
import RegionsMapping from './RegionsMapping';
import PlayerPositions from './PlayerPositions';
import CreepWaveLayer from './CreepWaveLayer';
import { Region } from '../../domain/region';
import { ScaledPlayerCoord, PlayerData } from '../../domain/player';
import { ScaledBossSnapshot } from '../../domain/boss';
import { DestroyedObjective } from '../../domain/destroyedObjective';
import { LaneCreepData } from '../../domain/creep';

const MINIMAP_URL =
  'https://assets-bucket.deadlock-api.com/assets-api-res/images/maps/minimap.png';

const Minimap = ({
  currentSecond,
  setCurrentSecond,
  match_duration_s,
  scaledBossSnapshots,
  MINIMAP_SIZE,
  destroyedObjectivesSorted,
  setCurrentObjectiveIndex,
  regions,
  scaledPlayerCoords,
  players,
  startRepeat,
  stopRepeat,
  laneCreepData,
  worldToMinimapPixels,
}: {
  currentSecond: number;
  setCurrentSecond: Dispatch<SetStateAction<number>>;
  match_duration_s: number;
  match_start_time_s: number;
  scaledBossSnapshots: ScaledBossSnapshot[];
  MINIMAP_SIZE: number;
  destroyedObjectivesSorted: Array<DestroyedObjective>;
  setCurrentObjectiveIndex: Dispatch<SetStateAction<number>>;
  regions: Region[];
  scaledPlayerCoords: ScaledPlayerCoord[];
  players: PlayerData[];
  startRepeat: (direction: 'back' | 'forward') => void;
  stopRepeat: () => void;
  laneCreepData: LaneCreepData;
  worldToMinimapPixels: (x: number, y: number) => { left: number; top: number };
}) => {
  const mapRef = useRef<HTMLImageElement>(null);
  const [activeObjectiveKey, setActiveObjectiveKey] = useState<string | null>(
    null
  );
  const [visibleRegions, setVisibleRegions] = useState<{
    [label: string]: boolean;
  }>(() => Object.fromEntries(regions.map((r) => [r.label, true])));
  const visibleRegionList = regions.filter((r) => visibleRegions[r.label]);

  const handleRegionToggle = (label: string) => {
    setVisibleRegions((v) => ({ ...v, [label]: !v[label] }));
  };

  useEffect(() => {
    let lastActiveKey: string | null = null;
    let currentIdx = -1;
    destroyedObjectivesSorted.forEach((obj, idx) => {
      if (currentSecond >= obj.destroyed_time_s) {
        lastActiveKey = `${obj.team}_${obj.team_objective_id}`;
        currentIdx = idx;
      }
    });
    setActiveObjectiveKey(lastActiveKey);
    setCurrentObjectiveIndex(currentIdx);
  }, [destroyedObjectivesSorted, currentSecond]);

  return (
    <>
      {/* Minimap and slider */}
      <div className='h-fit shadow shadow-black/50'>
        <div
          className={`pointer-events-none relative`}
          style={{ width: `${MINIMAP_SIZE}px`, height: `${MINIMAP_SIZE}px` }}
        >
          {/* <Grid MINIMAP_SIZE={MINIMAP_SIZE} /> */}
          <RegionsMapping
            MINIMAP_SIZE={MINIMAP_SIZE}
            regions={visibleRegionList}
          />
          <img
            ref={mapRef}
            src={MINIMAP_URL}
            alt='Minimap'
            className='pointer-events-none z-0 h-full w-full object-cover'
          />
          <Objectives
            scaledBossSnapshots={scaledBossSnapshots}
            destroyedObjectives={destroyedObjectivesSorted}
            currentTick={currentSecond}
            activeObjectiveKey={activeObjectiveKey}
          />
          <PlayerPositions
            scaledPlayerCoords={scaledPlayerCoords}
            players={players}
          />
          <CreepWaveLayer
            laneCreepData={laneCreepData}
            currentSec={currentSecond}
            worldToMinimapPixels={worldToMinimapPixels}
          />
        </div>
        <div className='border-top padding-0 flex w-full flex-col items-stretch gap-0 border-black/50 bg-gray-300'>
          <RegionToggle
            regions={regions}
            visibleRegions={visibleRegions}
            onToggle={handleRegionToggle}
          />
        </div>
      </div>
    </>
  );
};

export default Minimap;
