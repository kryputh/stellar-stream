import React from 'react';
import { render, screen } from '@testing-library/react';
import { describe, it, expect, vi } from 'vitest';
import { StreamsTable } from '../components/StreamsTable';
import { Stream } from '../types/stream';

const noop = vi.fn();

const mockStreams: Stream[] = [
  {
    id: '1',
    sender: 'G_SENDER',
    recipient: 'G_RECIPIENT123',
    assetCode: 'USDC',
    totalAmount: 100,
    durationSeconds: 3600,
    startAt: 1670000000,
    createdAt: 1670000000,
    progress: {
      status: 'active',
      ratePerSecond: 0.01,
      elapsedSeconds: 100,
      vestedAmount: 20,
      remainingAmount: 80,
      percentComplete: 20,
    },
  },
];

const defaultProps = {
  streams: mockStreams,
  filters: {},
  onFiltersChange: noop,
  onCancel: vi.fn().mockResolvedValue(undefined),
  onEditStartTime: noop,
};

describe('StreamsTable Component', () => {
  it('renders table data when streams are passed', () => {
    render(<StreamsTable {...defaultProps} />);
    // CopyableAddress truncates the display text, so match via title attribute
    expect(screen.getByTitle('G_RECIPIENT123')).toBeInTheDocument();
    expect(screen.getByText(/active/i)).toBeInTheDocument();
  });

  it('renders an empty state when no streams', () => {
    render(<StreamsTable {...defaultProps} streams={[]} />);
    expect(screen.queryByText(/G_RECIPIENT123/i)).not.toBeInTheDocument();
    expect(screen.getByText(/no streams match/i)).toBeInTheDocument();
  });

  it('renders a View button when onViewDetail is provided', () => {
    const onViewDetail = vi.fn();
    render(<StreamsTable {...defaultProps} onViewDetail={onViewDetail} />);
    expect(screen.getByTitle('View stream detail')).toBeInTheDocument();
  });

  it('calls onViewDetail with stream id when View is clicked', () => {
    const onViewDetail = vi.fn();
    render(<StreamsTable {...defaultProps} onViewDetail={onViewDetail} />);
    screen.getByTitle('View stream detail').click();
    expect(onViewDetail).toHaveBeenCalledWith('1');
  });
});