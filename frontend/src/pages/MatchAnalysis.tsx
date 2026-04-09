import { useState, useEffect, useMemo, useRef } from 'react';
import { useParams } from 'react-router-dom';
import Minimap from '../components/matchAnalysis/Minimap';
import PlayerCards from '../components/matchAnalysis/PlayerCards';
import ObjectiveInfoPanel from '../components/matchAnalysis/ObjectiveInfoPanel';
import TeamDisplay from '../components/matchAnalysis/TeamDisplay';
import MatchTimeViewer from '../components/matchAnalysis/MatchTimeViewer';
import DamageAnalysisSection from '../components/damageAnalysis/DamageAnalysisSection';
import { ErrorMessage } from '../components/common/ErrorMessage';
import { useErrorHandler } from '../hooks/useErrorHandler';
import { regions } from '../data/regions';
import { DestroyedObjective } from '../domain/destroyedObjective';
import { Hero, PlayerData, ScaledPlayerCoord } from '../domain/player';
import { ScaledBossSnapshot } from '../domain/boss';
import { ScaledSinnerSnapshot } from '../domain/sinner';
import { useMatchAnalysis } from '../hooks/UseMatchAnalysis';
import PrintHeroImageData from '../components/matchAnalysis/PrintHeroImageData';
import { formatSecondstoMMSS } from '../utils/time';

import {
  defaultMatchAnalysis,
  MatchAnalysisResponse,
  WORLD_BOUNDS,
} from '../domain/matchAnalysis';

const MINIMAP_SIZE = 768;
// FIXME: Use this value once we're confident in how our map
// looks at bigger sizes.
// const MINIMAP_SIZE = 512;

// Coordinate transformation functions
const normalizePosition = (x: number, y: number) => {
  const { xMin, xMax, yMin, yMax } = WORLD_BOUNDS;
  const spanX = Math.max(1e-6, xMax - xMin);
  const spanY = Math.max(1e-6, yMax - yMin);
  const normX = (x - xMin) / spanX;
  // Invert Y axis for minimap representation
  const normY = 1 - (y - yMin) / spanY;

  return { normX, normY };
};

const normToScaledPixels = (normX: number, normY: number) => {
  const xOffset = -10;
  const left = normX * MINIMAP_SIZE + xOffset; // Apply offset to x-coordinate
  const top = normY * MINIMAP_SIZE;
  return { left, top };
};

const worldToMinimapPixels = (x: number, y: number) => {
  const { normX, normY } = normalizePosition(x, y);
  return normToScaledPixels(normX, normY);
};

const MatchAnalysis = () => {
  const { match_id } = useParams();
  // Fetch match analysis via ETag-aware hook
  const {
    data: matchAnalysisData,
    loading,
    error: matchError,
    refetch,
  } = useMatchAnalysis(Number(match_id));
  const {
    error: heroError,
    handleError: handleHeroError,
    clearError: clearHeroError,
  } = useErrorHandler();

  const matchAnalysis: MatchAnalysisResponse =
    matchAnalysisData ?? defaultMatchAnalysis;
  // FIXME: matchMetadata is Deadlock API stuff that we'll likely get rid of later
  const matchMetadata = matchAnalysis.match_metadata;
  const parsedMatchData = matchAnalysis.parsed_match_data;
  const bossSnapshots = parsedMatchData.bosses.snapshots;
  const sinnerSnapshots = parsedMatchData.sinners;
  // NOTE: Contains player info
  const playersData = parsedMatchData.players_data;
  // NOTE: Contains dmg/position data per player
  const perPlayerData = parsedMatchData.per_player_data;
  const matchDuration = parsedMatchData.match_duration_s;
  const matchStartTime = parsedMatchData.match_start_time_s;
  const [heroData, setHeroData] = useState<Hero[]>([
    { id: 0, name: 'Default', images: {} },
  ]);
  const isMounted = useRef(false);

  const [currentTick, setCurrentTick] = useState<number>(0);
  const matchTime = formatSecondstoMMSS(currentTick + matchStartTime);

  // Timeline repeat functionality (for hold-to-scrub)
  const repeatRef = useRef<NodeJS.Timeout | null>(null);

  const startRepeat = (direction: 'back' | 'forward') => {
    if (repeatRef.current) return;
    repeatRef.current = setInterval(() => {
      setCurrentTick((t) => {
        if (direction === 'back') {
          if (t <= 0) return 0;
          return t - 1;
        } else {
          if (t >= matchDuration) return matchDuration;
          return t + 1;
        }
      });
    }, 80);
  };

  const stopRepeat = () => {
    if (repeatRef.current) {
      clearInterval(repeatRef.current);
      repeatRef.current = null;
    }
  };

  // Prepare destroyed objectives: filter out those with destroyed_time_s === 0 and sort by destroyed_time_s
  // NOTE: Unsure where the objectives with destroyed_time_s === 0 come from, but they are not useful for
  // the minimap. It may be worth revisiting later.
  const destroyedObjectivesSorted: Array<DestroyedObjective> =
    matchMetadata.match_info.objectives
      .filter((obj) => obj.destroyed_time_s !== 0)
      .sort((a, b) => a.destroyed_time_s - b.destroyed_time_s);
  const [currentObjectiveIndex, setCurrentObjectiveIndex] = useState(-1);

  const players: PlayerData[] = useMemo(() => {
    if (!playersData || !heroData) return [];
    const heroIdToHero: Record<number, Hero> = {};
    heroData.forEach((h) => {
      heroIdToHero[h.id] = h;
    });
    return playersData.map((player) => {
      const hero = heroIdToHero[player.hero_id] || {
        id: 0,
        name: 'Unknown',
        images: {},
      };
      // Enrich hero with specific image URLs
      return {
        ...player,
        hero: {
          ...hero,
          minimapImage: hero.images?.minimap_image_webp as string | undefined,
          heroCardWebp: hero.images?.icon_hero_card_webp as string | undefined,
        },
      };
    });
  }, [playersData, heroData]);

  const scaledPlayerCoords: ScaledPlayerCoord[] = useMemo(() => {
    return Object.entries(perPlayerData).map(([customId, playerMatchData]) => {
      const pos = playerMatchData.positions[currentTick];
      const { left, top } = worldToMinimapPixels(pos.x, pos.y);
      return {
        customId,
        x: pos.x,
        y: pos.y,
        z: pos.z,
        is_npc: pos.is_npc,
        left,
        top,
      };
    });
  }, [perPlayerData, currentTick]);

  const scaledBossSnapshots: ScaledBossSnapshot[] = useMemo(
    () =>
      bossSnapshots.map((snapshot) => ({
        ...snapshot,
        ...worldToMinimapPixels(snapshot.x, snapshot.y),
      })),
    [bossSnapshots]
  );

  const scaledSinnerSnapshots: ScaledSinnerSnapshot[] = useMemo(
    () =>
      sinnerSnapshots.map((snapshot) => ({
        ...snapshot,
        ...worldToMinimapPixels(snapshot.x, snapshot.y),
      })),
    [sinnerSnapshots]
  );

  // Diagnostic: log lane_creep_data stats once per load. Helps identify frozen-creep bugs:
  // - "sec-0 creeps" are spawned at match start (expected), but a high count hints at ghost entries
  // - "lane-0 wave IDs" mean the parser registered a creep before its lane was assigned (bug)
  // - window.__debugCreepAt(sec) dumps alive creeps at any second for manual inspection
  useEffect(() => {
    const data = parsedMatchData.lane_creep_data;
    if (!data || Object.keys(data.creeps).length === 0) return;

    const creepIds = Object.keys(data.creeps);
    const sec0Creeps = creepIds.filter((id) => {
      const t = data.creeps[id];
      return Array.isArray(t) && t.length > 0 && t[0] !== null;
    });
    const lane0Waves = Object.keys(data.wave_meta).filter((id) =>
      id.startsWith('0_')
    );

    console.group('[CreepData] Loaded');
    console.log(
      `Creeps: ${creepIds.length} total, ${sec0Creeps.length} with snapshot at sec 0`
    );
    console.log(`Waves: ${Object.keys(data.wave_meta).length} total`);
    if (lane0Waves.length > 0) {
      console.warn('Lane-0 wave IDs (parser bug indicator):', lane0Waves);
    }
    if (sec0Creeps.length > 0) {
      console.log('Creep IDs with sec-0 snapshot:', sec0Creeps);
    }
    console.groupEnd();

    if (import.meta.env.MODE === 'development') {
      (window as unknown as Record<string, unknown>).__debugCreepAt = (
        sec: number
      ) => {
        const alive = creepIds
          .filter((id) => {
            const t = data.creeps[id];
            return Array.isArray(t) && sec < t.length && t[sec] !== null;
          })
          .map((id) => ({ id, snapshot: data.creeps[id][sec] }));
        console.log(`[CreepData] sec ${sec}: ${alive.length} alive`, alive);
        return alive;
      };
    }
  }, [parsedMatchData.lane_creep_data]);

  useEffect(() => {
    isMounted.current = true;

    fetch('https://assets.deadlock-api.com/v2/heroes?only_active=true')
      .then((res) => res.json())
      .then((data) => {
        if (!isMounted.current) return;
        console.log('Loaded hero data:', data);
        setHeroData(data);
        clearHeroError();
      })
      .catch((err) => {
        if (!isMounted.current) return;
        console.error('Error fetching hero data:', err);
        handleHeroError(err);
      });

    return () => {
      isMounted.current = false;
    };
  }, [match_id, handleHeroError, clearHeroError]);

  return (
    <>
      <TeamDisplay players={players} />
      <div className='mx-auto flex flex-col gap-1 px-8'>
        <h2>Match ID: {match_id}</h2>
      </div>

      {/* Display match analysis errors */}
      {matchError && (
        <div className='mx-auto max-w-4xl px-8 py-4'>
          <ErrorMessage
            error={matchError as Error}
            title='Failed to Load Match Analysis'
            onRetry={refetch}
          />
        </div>
      )}

      {/* Display hero data errors */}
      {heroError && (
        <div className='mx-auto max-w-4xl px-8 py-4'>
          <ErrorMessage
            error={heroError}
            title='Failed to Load Hero Data'
          />
        </div>
      )}

      {/* Show loading state */}
      {loading && !matchAnalysisData && (
        <div className='mx-auto max-w-4xl px-8 py-4 text-center'>
          <p className='text-gray-600'>Loading match analysis...</p>
        </div>
      )}

      {/* Only show content if we have data and no errors */}
      {matchAnalysisData && !matchError && (
        <>
          <DamageAnalysisSection
            players={players}
            perPlayerData={perPlayerData}
            bossSnapshots={scaledBossSnapshots}
            totalMatchTime={matchDuration}
          />

          <MatchTimeViewer
            currentTick={currentTick}
            setCurrentTick={setCurrentTick}
            match_duration_s={matchDuration}
            match_start_time_s={matchStartTime}
            startRepeat={startRepeat}
            stopRepeat={stopRepeat}
          />

          <div className='match-analysis'>
            <div className='grid grid-cols-[1fr_47vw] gap-3'>
              <div
                title='InformationPanel'
                className='box-border gap-2 border-2 border-black bg-gray-300'
              >
                <ObjectiveInfoPanel
                  destroyedObjectives={destroyedObjectivesSorted}
                  currentObjectiveIndex={currentObjectiveIndex}
                />
                <PlayerCards
                  players={players}
                  perPlayerData={perPlayerData}
                  currentTick={currentTick}
                  normalizePosition={normalizePosition}
                  matchData={matchAnalysis.parsed_match_data}
                  lanePressure={parsedMatchData.lane_pressure}
                  sinners={parsedMatchData.sinners}
                />
              </div>
              <div className='flex flex-col gap-2'>
                <Minimap
                  currentSecond={currentTick}
                  setCurrentSecond={setCurrentTick}
                  match_duration_s={matchDuration}
                  match_start_time_s={
                    matchAnalysis.parsed_match_data.match_start_time_s
                  }
                  MINIMAP_SIZE={MINIMAP_SIZE}
                  scaledBossSnapshots={scaledBossSnapshots}
                  scaledPlayerCoords={scaledPlayerCoords}
                  players={players}
                  destroyedObjectivesSorted={destroyedObjectivesSorted}
                  setCurrentObjectiveIndex={setCurrentObjectiveIndex}
                  regions={regions}
                  startRepeat={startRepeat}
                  stopRepeat={stopRepeat}
                  laneCreepData={parsedMatchData.lane_creep_data}
                  worldToMinimapPixels={worldToMinimapPixels}
                  scaledSinnerSnapshots={scaledSinnerSnapshots}
                />
              </div>
            </div>
          </div>

          {/* Player combat type/health Table */}
          {/*
          The buttons that control the time windows has been removed, but I may have use for some of the
          code in here so I'm keeping it for now
        */}
          {/* <PerPlayerWindowTable
          playerPaths={playerPaths}
          matchMetadata={matchMetadata}
          playerTime={playerTime}
          heros={heros}
        /> */}

          {/* Damage Source Types Table */}
          {/* <DamageSourceTypesTable
          sourceDetails={matchMetadata.match_info.damage_matrix.source_details}
        /> */}

          {/* Digestible Damage Matrix Table for Abrams (player_slot 1) */}
          {/* <DamageMatrixTable matchMetadata={matchMetadata} /> */}

          {/* <PrintHeroImageData heroData={heroData} /> */}
        </>
      )}
    </>
  );
};

export default MatchAnalysis;
