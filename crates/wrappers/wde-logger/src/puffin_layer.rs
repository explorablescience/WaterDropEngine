#![cfg(feature = "puffin")]

use std::{cell::RefCell, collections::VecDeque};

use bevy::prelude::*;
use puffin::ThreadProfiler;
use tracing::{
    Id, Subscriber,
    span::{Attributes, Record}
};
use tracing_subscriber::{
    Layer,
    fmt::{FormatFields, FormattedFields, format::DefaultFields},
    layer::Context,
    registry::LookupSpan
};

thread_local! {
    static PUFFIN_SPAN_STACK: RefCell<VecDeque<(Id, usize)>> =
        RefCell::new(VecDeque::with_capacity(16));
}

pub fn puffin_new_frame_system() {
    puffin::profile_function!();
    puffin::GlobalProfiler::lock().new_frame();
}

struct PuffinScopeId(puffin::ScopeId);

/// A tracing layer that collects data for puffin.
pub struct PuffinLayer<F = DefaultFields> {
    fmt: F
}
impl Default for PuffinLayer<DefaultFields> {
    fn default() -> Self {
        Self::new()
    }
}
impl PuffinLayer<DefaultFields> {
    pub fn new() -> Self {
        Self {
            fmt: DefaultFields::default()
        }
    }
}
impl<S: Subscriber, F> Layer<S> for PuffinLayer<F>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    F: for<'writer> FormatFields<'writer> + 'static
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();

            if extensions.get_mut::<PuffinScopeId>().is_none() {
                let metadata = span.metadata();
                let scope_name = metadata.name();
                let function_name = metadata.target();
                let file_path = metadata.file().unwrap_or("unknown");
                let line_nr = metadata.line().unwrap_or(0);
                let scope_id = ThreadProfiler::call(|tp| {
                    tp.register_named_scope(scope_name, function_name, file_path, line_nr)
                });
                extensions.insert(PuffinScopeId(scope_id));
            }

            if extensions.get_mut::<FormattedFields<F>>().is_none() {
                let mut fields = FormattedFields::<F>::new(String::with_capacity(64));
                if self.fmt.format_fields(fields.as_writer(), attrs).is_ok() {
                    extensions.insert(fields);
                }
            }
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(fields) = extensions.get_mut::<FormattedFields<F>>() {
                let _ = self.fmt.add_fields(fields, values);
            } else {
                let mut fields = FormattedFields::<F>::new(String::with_capacity(64));
                if self.fmt.format_fields(fields.as_writer(), values).is_ok() {
                    extensions.insert(fields);
                }
            }
        }
    }

    fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
        if !puffin::are_scopes_on() {
            return;
        }

        if let Some(span_data) = ctx.span(id) {
            let extensions = span_data.extensions();
            let scope_id = extensions.get::<PuffinScopeId>().map(|scope| scope.0);
            let data = extensions
                .get::<FormattedFields<F>>()
                .map(|fields| fields.fields.as_str())
                .unwrap_or_default();

            if let Some(scope_id) = scope_id {
                ThreadProfiler::call(|tp| {
                    let start_stream_offset = tp.begin_scope(scope_id, data);
                    PUFFIN_SPAN_STACK.with(|s| {
                        s.borrow_mut().push_back((id.clone(), start_stream_offset));
                    });
                });
            }
        }
    }

    fn on_exit(&self, id: &Id, _ctx: Context<'_, S>) {
        PUFFIN_SPAN_STACK.with(|s| {
            let value = s.borrow_mut().pop_back();
            if let Some((last_id, start_stream_offset)) = value {
                if *id == last_id {
                    ThreadProfiler::call(|tp| tp.end_scope(start_stream_offset));
                } else {
                    s.borrow_mut().push_back((last_id, start_stream_offset));
                }
            }
        });
    }

    fn on_close(&self, id: Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(&id) {
            span.extensions_mut().remove::<PuffinScopeId>();
            span.extensions_mut().remove::<FormattedFields<F>>();
        }
    }
}
