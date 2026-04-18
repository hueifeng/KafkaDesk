type SignalStatProps = {
  label: string;
  value: string;
  note: string;
};

export function SignalStat({ label, value, note }: SignalStatProps) {
  return (
    <article className="signal-stat">
      <p className="signal-stat-label">{label}</p>
      <p className="signal-stat-value">{value}</p>
      <p className="signal-stat-note">{note}</p>
    </article>
  );
}
