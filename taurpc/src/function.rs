use std::borrow::Cow;

use specta::datatype::DataType;

/// The runtime type information for a single procedure, used for generating the TypeScript bindings.
///
/// Values are produced by the `#[taurpc::procedures]` macro and collected through
/// [`TauRpcFunction`](crate::TauRpcFunction).
#[derive(Debug, Clone)]
pub struct Function {
    name: Cow<'static, str>,
    args: Vec<(Cow<'static, str>, DataType)>,
    result: Option<DataType>,
    docs: Cow<'static, str>,
}

impl Function {
    /// Constructs the type information for a procedure.
    ///
    /// This is only public so the `#[taurpc::procedures]` macro can construct it on your behalf.
    #[doc(hidden)]
    pub fn new(
        name: impl Into<Cow<'static, str>>,
        args: Vec<(Cow<'static, str>, DataType)>,
        result: Option<DataType>,
        docs: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            name: name.into(),
            args,
            result,
            docs: docs.into(),
        }
    }

    /// The name of the procedure, as it appears in the TypeScript router.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The name and type of each procedure argument that is sent from the frontend.
    pub fn args(&self) -> &[(Cow<'static, str>, DataType)] {
        &self.args
    }

    /// The return type of the procedure, or `None` if it returns nothing.
    pub fn result(&self) -> Option<&DataType> {
        self.result.as_ref()
    }

    /// The documentation of the procedure, taken from its Rust doc comments.
    pub fn docs(&self) -> &str {
        &self.docs
    }
}
