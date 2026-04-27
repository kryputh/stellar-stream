# Wave 4 — StellarStream Implementation Backlog

Wave 4 focuses on **production hardening, advanced streaming features, DeFi integrations, and developer experience** improvements across all layers of the stack.

## Themes

- **Contract**: cliff vesting, multi-recipient, pause/resume, compliance clawback
- **Backend**: retry logic, webhook integrity, indexer reliability, auth hardening
- **Frontend**: real-time WebSocket updates, dark mode, bulk operations, richer UX
- **Infra**: Docker healthchecks, CI improvements, observability

## File Coverage

| Area | Files Targeted |
|------|---------------|
| Smart Contract | `contracts/src/lib.rs`, `contracts/src/test.rs` |
| Backend Core | `backend/src/index.ts`, `backend/src/services/streamStore.ts` |
| Backend Services | `backend/src/services/auth.ts`, `backend/src/services/eventHistory.ts` |
| Backend Infra | `backend/src/services/indexer.ts`, `backend/src/services/webhook.ts` |
| Validation | `backend/src/validation/schemas.ts`, `backend/src/swagger.ts` |
| Frontend | `frontend/src/App.tsx`, `frontend/src/components/*` |
| Hooks | `frontend/src/hooks/*` |
| Infra | `docker-compose.yml`, `.github/workflows/ci.yml` |

## Issue Summary (50 items)

Issues are grouped by file and tagged with labels: `wave4`, `contract`, `backend`, `frontend`, `infra`, `auth`, `testing`.

---

## Wave 4 Checklist

- [ ] Cliff vesting in `create_stream`
- [ ] Multi-recipient split streams
- [ ] Stream pause/resume contract functions
- [ ] Compliance clawback mechanism
- [ ] Native XLM streaming support
- [ ] On-chain stream metadata
- [ ] Stream transfer (change recipient)
- [ ] Richer `StreamCreated` event payload
- [ ] `get_claimable_batch` for multiple IDs
- [ ] SAC validation in `create_stream`
- [ ] Fuzz tests for `vested_amount`
- [ ] Cancel-then-claim integration test
- [ ] Concurrent claim test
- [ ] `StreamCreated` event snapshot test
- [ ] Boundary tests for `claimable`
- [ ] RPC response caching in `syncStreams`
- [ ] Retry with exponential backoff in `createStream`
- [ ] Cancel refund amount from on-chain response
- [ ] `pauseStream` / `resumeStream` backend functions
- [ ] Batch `get_stream` in `syncStreams`
- [ ] Stream archival after 30 days
- [ ] Configurable cron for `refreshStreamStatuses`
- [ ] JWT secret rotation warning on startup
- [ ] Refresh token endpoint
- [ ] Rate limiting on `/api/auth/challenge`
- [ ] Multi-sig challenge verification
- [ ] Pagination for `getStreamHistory`
- [ ] Record `claimed` events from contract
- [ ] Event deduplication by ledger sequence
- [ ] Aggregated event counts endpoint
- [ ] Cursor-based pagination in indexer
- [ ] Circuit breaker for RPC failures
- [ ] Metrics via `/api/metrics`
- [ ] Configurable start ledger for backfill
- [ ] Query param validation on CSV export
- [ ] `DELETE /api/streams/:id` admin endpoint
- [ ] Request ID in all error responses
- [ ] `GET /api/stats` aggregate endpoint
- [ ] Stellar account ID validation in schemas
- [ ] Webhook registration schema
- [ ] HMAC-SHA256 webhook signatures
- [ ] Webhook retry queue with dead-letter storage
- [ ] Dark mode toggle with localStorage
- [ ] WebSocket real-time updates replacing polling
- [ ] Global toast notification system
- [ ] Asset selector dropdown in CreateStreamForm
- [ ] End date/time preview in CreateStreamForm
- [ ] Column sorting in StreamsTable
- [ ] Bulk cancel in StreamsTable
- [ ] Zoom/pan controls in StreamMetricsChart
