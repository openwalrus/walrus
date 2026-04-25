//! Session search benchmark — verifies the RFC 0185 performance budget.
//!
//! Targets:
//! - `search` p99 ≤ 50ms at 100k indexed messages (p99 ≤ 200ms at 1M).
//! - `insert_message` ≤ 1ms at any scale up to 1M messages.
//! - Cold-start `rebuild` ≤ 500ms at 100k messages.
//!
//! `criterion` reports mean times by default; we run a few sizes so a
//! regression sticks out as a step change.

use crabtalk_bench::generate_corpus;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use runtime::sessions::{SearchOptions, SessionIndex};
use wcore::{model::HistoryEntry, storage::SessionHandle};

const SESSION_SIZE: usize = 50; // messages per synthetic session

fn build_index(messages: usize) -> SessionIndex {
    let mut idx = SessionIndex::new();
    let corpus = generate_corpus(messages);
    let session_count = (messages / SESSION_SIZE).max(1);
    let now = "2026-04-25T00:00:00Z";

    let mut session_ids = Vec::with_capacity(session_count);
    for s in 0..session_count {
        let handle = SessionHandle::new(format!("crab_tester_{s}"));
        let id = idx.ensure_session(&handle, "crab", "tester", "", None, now, now);
        session_ids.push(id);
    }
    for (i, (_doc_idx, text)) in corpus.into_iter().enumerate() {
        let session_id = session_ids[i % session_count];
        idx.insert_message(session_id, &HistoryEntry::user(text));
    }
    idx
}

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_search/search");
    for messages in [1_000usize, 10_000, 100_000] {
        let idx = build_index(messages);
        group.bench_with_input(
            BenchmarkId::from_parameter(messages),
            &idx,
            |b, idx: &SessionIndex| {
                b.iter(|| idx.search("agent memory recall session", &SearchOptions::default()));
            },
        );
    }
    group.finish();
}

fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_search/insert_message");
    for prepop in [0usize, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(prepop),
            &prepop,
            |b, &prepop| {
                let mut idx = build_index(prepop);
                let handle = SessionHandle::new("crab_tester_live");
                let now = "2026-04-25T00:00:00Z";
                let session_id = idx.ensure_session(&handle, "crab", "tester", "", None, now, now);
                let entry = HistoryEntry::user("a fresh message arrives with words to index");
                b.iter(|| {
                    idx.insert_message(session_id, &entry);
                });
            },
        );
    }
    group.finish();
}

fn bench_rebuild(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_search/rebuild");
    for messages in [10_000usize, 100_000] {
        // The "rebuild" we benchmark is reconstructing the index from
        // pre-tokenized messages — i.e., the CPU work that happens
        // after `Storage::list_sessions` returns. Storage I/O is
        // measured separately if it ever becomes interesting.
        group.bench_with_input(
            BenchmarkId::from_parameter(messages),
            &messages,
            |b, &messages| {
                let corpus = generate_corpus(messages);
                let session_count = (messages / SESSION_SIZE).max(1);
                let now = "2026-04-25T00:00:00Z";
                b.iter(|| {
                    let mut idx = SessionIndex::new();
                    let mut session_ids = Vec::with_capacity(session_count);
                    for s in 0..session_count {
                        let handle = SessionHandle::new(format!("crab_tester_{s}"));
                        let id = idx.ensure_session(&handle, "crab", "tester", "", None, now, now);
                        session_ids.push(id);
                    }
                    for (i, (_, text)) in corpus.iter().enumerate() {
                        let session_id = session_ids[i % session_count];
                        idx.insert_message(session_id, &HistoryEntry::user(text.clone()));
                    }
                    idx
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_search, bench_insert, bench_rebuild);
criterion_main!(benches);
