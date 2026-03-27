import { useEffect, useState, useCallback } from "react";
import { getStreamHistory, listAllEvents, StreamEvent } from "../services/api";

interface StreamTimelineProps {
  streamId?: string;
}

/** Simple "time ago" formatter */
function timeAgo(timestamp: number): string {
  const seconds = Math.floor(Date.now() / 1000 - timestamp);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

function getEventIcon(eventType: string): string {
  switch (eventType) {
    case "created":          return "🚀";
    case "claimed":          return "💸";
    case "canceled":         return "❌";
    case "start_time_updated": return "🕐";
    default:                 return "📋";
  }
}

function getEventDescription(event: StreamEvent): string {
  const actor = event.actor
    ? `${event.actor.slice(0, 6)}...${event.actor.slice(-4)}`
    : "Unknown";
  switch (event.eventType) {
    case "created":
      return `Initiated by ${actor} for ${event.amount} tokens`;
    case "claimed":
      return `Claim of ${event.amount} tokens processed by ${actor}`;
    case "canceled":
      return `Closed by ${actor}`;
    case "start_time_updated":
      return `New start time set by ${actor}`;
    default:
      return `Action performed by ${actor}`;
  }
}

export function StreamTimeline({ streamId }: StreamTimelineProps) {
  const [events, setEvents] = useState<StreamEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadHistory = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = streamId
        ? await getStreamHistory(streamId)
        : await listAllEvents();
      setEvents(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load history.");
    } finally {
      setLoading(false);
    }
  }, [streamId]);

  useEffect(() => {
    loadHistory();
  }, [loadHistory]);

  if (loading) return <p className="muted">Loading timeline…</p>;
  if (error)   return <p className="muted">Error: {error}</p>;
  if (events.length === 0) return <p className="muted">No events yet.</p>;

  return (
    <div className="activity-feed">
      {events.map((event) => (
        <div key={event.id} className="activity-item">
          <div className="activity-icon">{getEventIcon(event.eventType)}</div>
          <div>
            <div style={{ fontWeight: 600, fontSize: "0.88rem" }}>
              {event.eventType.replace(/_/g, " ")}
            </div>
            <div className="muted">{getEventDescription(event)}</div>
            <div className="muted" style={{ fontSize: "0.75rem" }}>
              {timeAgo(event.timestamp)}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
