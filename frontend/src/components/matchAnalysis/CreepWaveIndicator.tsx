interface CreepWaveIndicatorProps {
  left: number;
  top: number;
  mode: "live" | "pin";
  team: number;
}

const TEAM_COLORS: Record<number, string> = {
  2: "#FFA500", // Amber orange
  3: "#0EA5E9", // Sapphire blue
};

const CreepWaveIndicator = ({
  left,
  top,
  mode,
  team,
}: CreepWaveIndicatorProps) => {
  const fillColor =
    mode === "pin" ? "#000000" : (TEAM_COLORS[team] ?? "#888888");

  const radius = mode === "pin" ? 4 : 3;

  return (
    <div
      className="pointer-events-none absolute z-20"
      style={{
        left: `${left}px`,
        top: `${top}px`,
        transform: "translate(-50%, -50%)",
      }}
    >
      <svg
        width={radius * 2 + 2}
        height={radius * 2 + 2}
        viewBox={`0 0 ${radius * 2 + 2} ${radius * 2 + 2}`}
      >
        <circle
          cx={radius + 1}
          cy={radius + 1}
          r={radius}
          fill={fillColor}
        />
      </svg>
    </div>
  );
};

export default CreepWaveIndicator;
