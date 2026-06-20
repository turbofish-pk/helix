use std::sync::Arc;

use helix_core::{Rope, RopeSlice};
use imara_diff::{IndentHeuristic, IndentLevel, InternedInput};
use parking_lot::RwLock;
use tokio::sync::Notify;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::{Duration, timeout, timeout_at};

use crate::diff::{
    ALGORITHM, DIFF_DEBOUNCE_TIME_ASYNC, DIFF_DEBOUNCE_TIME_SYNC, DiffInner, Event, RenderLock,
};

use super::line_cache::InternedRopeLines;

#[cfg(test)]
mod test;

pub(super) struct DiffWorker {
    pub channel: UnboundedReceiver<Event>,
    pub diff: Arc<RwLock<DiffInner>>,
    pub diff_finished_notify: Arc<Notify>,
    pub diff_alloc: imara_diff::Diff,
}

impl DiffWorker {
    async fn accumulate_events(&mut self, event: Event) -> (Option<Rope>, Option<Rope>) {
        let mut accumulator = EventAccumulator::new();
        accumulator.handle_event(event);
        accumulator
            .accumulate_debounced_events(&mut self.channel, self.diff_finished_notify.clone())
            .await;
        (accumulator.doc, accumulator.diff_base)
    }

    pub async fn run(mut self, diff_base: Rope, doc: Rope) {
        let mut interner = InternedRopeLines::new(diff_base, doc);
        if let Some(lines) = interner.interned_lines() {
            self.perform_diff(lines);
        }
        self.apply_hunks(interner.diff_base(), interner.doc());
        while let Some(event) = self.channel.recv().await {
            let (doc_opt, diff_base_opt) = self.accumulate_events(event).await;

            let process_accumulated_events = || {
                // We must move/take the ropes out of the Options if we use them
                if let Some(new_base) = diff_base_opt {
                    // if let Some(doc) = doc_opt {
                    //     interner.update_diff_base(new_base, doc);
                    // }
                    interner.update_diff_base(new_base, doc_opt);
                } else if let Some(d) = doc_opt {
                    interner.update_doc(d);
                }

                if let Some(lines) = interner.interned_lines() {
                    self.perform_diff(lines);
                }
            };

            #[cfg(test)]
            process_accumulated_events();
            #[cfg(not(test))]
            tokio::task::block_in_place(process_accumulated_events);

            self.apply_hunks(interner.diff_base(), interner.doc());
        }
    }

    fn apply_hunks(&mut self, diff_base: Rope, doc: Rope) {
        let mut diff = self.diff.write();
        diff.diff_base = diff_base;
        diff.doc = doc;
        diff.hunks.clear();
        diff.hunks.extend(self.diff_alloc.hunks());
        drop(diff);
        self.diff_finished_notify.notify_waiters();
    }

    fn perform_diff(&mut self, input: &InternedInput<RopeSlice>) {
        self.diff_alloc.compute_with(
            ALGORITHM,
            &input.before,
            &input.after,
            input.interner.num_tokens(),
        );
        self.diff_alloc.postprocess_with(
            &input.before,
            &input.after,
            IndentHeuristic::new(|token| {
                IndentLevel::for_ascii_line(input.interner[token].bytes(), 4)
            }),
        );
    }
}

struct EventAccumulator {
    diff_base: Option<Rope>,
    doc: Option<Rope>,
    render_lock: Option<RenderLock>,
}

impl EventAccumulator {
    fn new() -> Self {
        Self {
            diff_base: None,
            doc: None,
            render_lock: None,
        }
    }

    fn handle_event(&mut self, event: Event) {
        let dst = if event.is_base {
            &mut self.diff_base
        } else {
            &mut self.doc
        };

        *dst = Some(event.text);

        if let Some(render_lock) = event.render_lock {
            match &mut self.render_lock {
                Some(RenderLock { timeout, .. }) => {
                    if render_lock.timeout.is_none() {
                        *timeout = None;
                    }
                }
                None => self.render_lock = Some(render_lock),
            }
        }
    }

    async fn accumulate_debounced_events(
        &mut self,
        channel: &mut UnboundedReceiver<Event>,
        diff_finished_notify: Arc<Notify>,
    ) {
        let async_debounce = Duration::from_millis(DIFF_DEBOUNCE_TIME_ASYNC);
        let sync_debounce = Duration::from_millis(DIFF_DEBOUNCE_TIME_SYNC);
        loop {
            let debounce = if self.render_lock.is_none() {
                async_debounce
            } else {
                sync_debounce
            };

            if let Ok(Some(event)) = timeout(debounce, channel.recv()).await {
                self.handle_event(event);
            } else {
                break;
            }
        }

        match self.render_lock.take() {
            None => {
                tokio::spawn(async move {
                    diff_finished_notify.notified().await;
                    helix_event::request_redraw();
                });
            }
            Some(RenderLock {
                lock,
                timeout: Some(timeout),
            }) => {
                tokio::spawn(async move {
                    let res = timeout_at(timeout, diff_finished_notify.notified()).await;
                    drop(lock);
                    if res.is_ok() {
                        return;
                    }
                    log::info!("Diff computation timed out, update of diffs might appear delayed");
                    diff_finished_notify.notified().await;
                    helix_event::request_redraw();
                });
            }
            Some(RenderLock {
                lock,
                timeout: None,
            }) => {
                tokio::spawn(async move {
                    diff_finished_notify.notified().await;
                    drop(lock);
                });
            }
        }
    }
}
