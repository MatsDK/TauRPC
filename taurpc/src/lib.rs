use serde::Deserializer;
use tauri::{Invoke, InvokeError, InvokeMessage, Runtime};

pub use serde::Serialize;
pub use ts_rs::TS;

pub use taurpc_macros::{procedures, rpc_struct};

pub trait TauRpcHandler<R: Runtime> {
    fn generate_ts_types();

    fn handle_incoming_request(self, invoke: Invoke<R>);
}

/// Accepts procedure resolver struct for which `taurpc::procedures` is implemented.
///
///  # Examples
/// ```rust
/// #[taurpc::procedures]
/// trait Api {
///     fn hello_world();
/// }
///
/// struct ApiImpl;
/// impl Api for ApiImpl {
///     fn hello_world(self) {
///         println!("Hello world");
///     }
/// }
///
/// fn main() {
///   let _handler = taurpc::create_rpc_handler(ApiImpl.into_handler());
/// }
/// ```
pub fn create_rpc_handler<H, R>(procedures: H) -> impl Fn(Invoke<R>) + Send + Sync + 'static
where
    H: TauRpcHandler<R> + Send + Sync + 'static + Clone,
    R: Runtime,
{
    H::generate_ts_types();

    move |invoke: Invoke<R>| {
        let cmd = invoke.message.command();
        println!("{cmd}");
        procedures.clone().handle_incoming_request(invoke);
    }
}

use serde::de::Visitor;
use serde::Deserialize;

/// Represents a custom command.
pub struct CommandItem<'a, R: tauri::Runtime> {
    /// The name of the command, e.g. `handler` on `#[command] fn handler(value: u64)`
    pub name: &'static str,

    /// The key of the command item, e.g. `value` on `#[command] fn handler(value: u64)`
    pub key: &'static str,

    pub idx: usize,

    /// The [`InvokeMessage`] that was passed to this command.
    pub message: &'a InvokeMessage<R>,
}

/// Trait implemented by command arguments to derive a value from a [`CommandItem`].
///
/// # Command Arguments
///
/// A command argument is any type that represents an item parsable from a [`CommandItem`]. Most
/// implementations will use the data stored in [`InvokeMessage`] since [`CommandItem`] is mostly a
/// wrapper around it.
///
/// # Provided Implementations
///
/// Tauri implements [`CommandArg`] automatically for a number of types.
/// * [`crate::Window`]
/// * [`crate::State`]
/// * `T where T: serde::Deserialize`
///   * Any type that implements `Deserialize` can automatically be used as a [`CommandArg`].
pub trait CommandArg<'de, R: Runtime>: Sized {
    /// Derives an instance of `Self` from the [`CommandItem`].
    ///
    /// If the derivation fails, the corresponding message will be rejected using [`InvokeMessage#reject`].
    fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError>;
}

/// Automatically implement [`CommandArg`] for any type that can be deserialized.
impl<'de, D: Deserialize<'de>, R: Runtime> CommandArg<'de, R> for D {
    fn from_command(command: CommandItem<'de, R>) -> Result<D, InvokeError> {
        let name = command.name;
        let arg = command.key;
        Self::deserialize(command).map_err(|e| tauri::Error::InvalidArgs(name, arg, e).into())
    }
}

/// Pass the result of [`serde_json::Value::get`] into [`serde_json::Value`]'s deserializer.
///
/// Returns an error if the [`CommandItem`]'s key does not exist in the value.
macro_rules! pass {
  ($fn:ident, $($arg:ident: $argt:ty),+) => {
    fn $fn<V: Visitor<'de>>(self, $($arg: $argt),*) -> Result<V::Value, Self::Error> {
      use serde::de::Error;

      if self.key.is_empty() {
        return Err(serde_json::Error::custom(format!(
            "command {} has an argument with no name with a non-optional value",
            self.name
          )))
      };

      let args = self.message.payload().get("args").unwrap().as_array().unwrap();
      args[self.idx].clone().$fn($($arg),*)
    }
  }
}

/// A [`Deserializer`] wrapper around [`CommandItem`].
///
/// If the key doesn't exist, an error will be returned if the deserialized type is not expecting
/// an optional item. If the key does exist, the value will be called with
/// [`Value`](serde_json::Value)'s [`Deserializer`] implementation.
impl<'de, R: Runtime> Deserializer<'de> for CommandItem<'de, R> {
    type Error = serde_json::Error;

    pass!(deserialize_any, visitor: V);
    pass!(deserialize_bool, visitor: V);
    pass!(deserialize_i8, visitor: V);
    pass!(deserialize_i16, visitor: V);
    pass!(deserialize_i32, visitor: V);
    pass!(deserialize_i64, visitor: V);
    pass!(deserialize_u8, visitor: V);
    pass!(deserialize_u16, visitor: V);
    pass!(deserialize_u32, visitor: V);
    pass!(deserialize_u64, visitor: V);
    pass!(deserialize_f32, visitor: V);
    pass!(deserialize_f64, visitor: V);
    pass!(deserialize_char, visitor: V);
    pass!(deserialize_str, visitor: V);
    pass!(deserialize_string, visitor: V);
    pass!(deserialize_bytes, visitor: V);
    pass!(deserialize_byte_buf, visitor: V);

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.message.payload().get(self.key) {
            Some(value) => value.deserialize_option(visitor),
            None => visitor.visit_none(),
        }
    }

    pass!(deserialize_unit, visitor: V);
    pass!(deserialize_unit_struct, name: &'static str, visitor: V);
    pass!(deserialize_newtype_struct, name: &'static str, visitor: V);
    pass!(deserialize_seq, visitor: V);
    pass!(deserialize_tuple, len: usize, visitor: V);

    pass!(
        deserialize_tuple_struct,
        name: &'static str,
        len: usize,
        visitor: V
    );

    pass!(deserialize_map, visitor: V);

    pass!(
        deserialize_struct,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V
    );

    pass!(
        deserialize_enum,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V
    );

    pass!(deserialize_identifier, visitor: V);
    pass!(deserialize_ignored_any, visitor: V);
}
