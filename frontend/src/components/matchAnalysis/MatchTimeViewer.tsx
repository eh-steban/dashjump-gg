import React from 'react';
import { formatSecondstoMMSS } from '../../utils/time';

interface MatchTimeViewerProps {
  currentTick: number;
  setCurrentTick: (time: number | ((t: number) => number)) => void;
  match_duration_s: number;
  match_start_time_s: number;
  startRepeat: (direction: 'back' | 'forward') => void;
  stopRepeat: () => void;
}

const MatchTimeViewer: React.FC<MatchTimeViewerProps> = ({
  currentTick,
  setCurrentTick,
  match_duration_s,
  match_start_time_s,
  startRepeat,
  stopRepeat,
}) => {
  // Display times offset by match_start_time_s so the clock matches the
  // in-game timer (e.g. positions[0] shows as "0:38" not "0:00").
  const currentTime = formatSecondstoMMSS(currentTick + match_start_time_s);
  const totalTime = formatSecondstoMMSS(match_duration_s + match_start_time_s);

  return (
    <div className='flex w-full justify-center'>
      <div className='w-full max-w-md rounded-lg bg-white p-4 shadow-md md:max-w-lg lg:max-w-xl'>
        {/* Time Display */}
        <div className='mb-3 text-center font-semibold text-gray-700'>
          {currentTime} / {totalTime}
        </div>

        {/* Controls */}
        <div className='flex items-center justify-center gap-3'>
          <button
            onClick={() => setCurrentTick((t) => Math.max(0, t - 1))}
            onMouseDown={() => startRepeat('back')}
            onMouseUp={stopRepeat}
            onMouseLeave={stopRepeat}
            onTouchStart={() => startRepeat('back')}
            onTouchEnd={stopRepeat}
            disabled={currentTick <= 0}
            className={`rounded border px-3 py-1 text-lg transition-colors ${
              currentTick <= 0 ?
                'cursor-not-allowed border-gray-300 bg-gray-200 text-gray-400'
              : 'cursor-pointer border-gray-300 bg-white text-gray-700 shadow-sm hover:bg-gray-100'
            }`}
            aria-label='Previous Tick'
          >
            ◀
          </button>

          <button
            onClick={() =>
              setCurrentTick((t) => Math.min(match_duration_s, t))
            }
            onMouseDown={() => startRepeat('forward')}
            onMouseUp={stopRepeat}
            onMouseLeave={stopRepeat}
            onTouchStart={() => startRepeat('forward')}
            onTouchEnd={stopRepeat}
            disabled={currentTick >= match_duration_s}
            className={`rounded border px-3 py-1 text-lg transition-colors ${
              currentTick >= match_duration_s ?
                'cursor-not-allowed border-gray-300 bg-gray-200 text-gray-400'
              : 'cursor-pointer border-gray-300 bg-white text-gray-700 shadow-sm hover:bg-gray-100'
            }`}
            aria-label='Next Tick'
          >
            ▶
          </button>

          <input
            type='range'
            min={0}
            max={match_duration_s}
            value={currentTick}
            onChange={(e) => setCurrentTick(Number(e.target.value))}
            className='h-2 flex-1 cursor-pointer appearance-none rounded-lg bg-gray-200 accent-blue-600'
          />
        </div>
      </div>
    </div>
  );
};

export default MatchTimeViewer;
