import { Badge } from '@/components/ui/badge';
import { DECODE_STATUS_DEFINITIONS } from '@/features/messages/decode-status';

type DecodeStatusLegendProps = {
  compact?: boolean;
};

export function DecodeStatusLegend({ compact = false }: DecodeStatusLegendProps) {
  return (
    <details className="workspace-block">
      <summary className="cursor-pointer list-row-title select-none">解码状态说明</summary>
      <div className={`list-stack ${compact ? 'mt-3' : 'mt-4'}`}>
        {DECODE_STATUS_DEFINITIONS.map((item) => (
            <div key={item.status} className="list-row">
              <div>
                <p className="list-row-title flex flex-wrap items-center gap-2">
                  <Badge tone={item.tone}>{item.label}</Badge>
                  <span className="font-mono text-xs text-ink-dim">{item.status}</span>
                </p>
                <p className="list-row-meta mt-2">{item.description}</p>
              </div>
          </div>
        ))}
      </div>
    </details>
  );
}
