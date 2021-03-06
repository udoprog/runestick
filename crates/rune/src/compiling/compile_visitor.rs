use runestick::{CompileMeta, SourceId, Span};

/// A visitor that will be called for every language item compiled.
pub trait CompileVisitor {
    /// Called when a meta item is registered.
    fn register_meta(&self, _meta: &CompileMeta) {}

    /// Mark that we've encountered a specific compile meta at the given span.
    fn visit_meta(&self, _source_id: SourceId, _meta: &CompileMeta, _span: Span) {}

    /// Visit a variable use.
    fn visit_variable_use(&self, _source_id: SourceId, _var_span: Span, _span: Span) {}

    /// Visit something that is a module.
    fn visit_mod(&self, _source_id: SourceId, _span: Span) {}
}

/// A compile visitor that does nothing.
pub struct NoopCompileVisitor(());

impl NoopCompileVisitor {
    /// Construct a new noop compile visitor.
    pub const fn new() -> Self {
        Self(())
    }
}

impl CompileVisitor for NoopCompileVisitor {}
