//! The procedure metadata is built by the `#[taurpc::procedures]` macro rather than
//! `specta::function`, so these assert the exact values that end up in the bindings.

use taurpc::specta::datatype::{DataType, Primitive, Reference};
use taurpc::{Exportable, TauRpcFunction};

#[taurpc::procedures]
trait Api {
    /// Update the state
    async fn update_state(new_value: String);

    ///
    /// Leading blank doc line
    async fn blank_first_line();

    #[taurpc(alias = "renamed")]
    fn original(amount: i32) -> i32;

    #[taurpc(event)]
    async fn state_changed(new_state: String);
}

#[derive(Clone)]
struct ApiImpl;

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn update_state(self, _new_value: String) {}

    async fn blank_first_line(self) {}

    fn original(self, amount: i32) -> i32 {
        amount
    }
}

fn procedure<'a>(procedures: &'a [TauRpcFunction], name: &str) -> &'a TauRpcFunction {
    procedures
        .iter()
        .find(|procedure| procedure.function.name() == name)
        .unwrap_or_else(|| panic!("procedure `{name}` was not collected"))
}

#[test]
fn collects_procedure_metadata() {
    let (types, procedures, args_map) = ApiImpl.into_handler().generate_types();
    let procedures = &procedures[""];

    let names = procedures
        .iter()
        .map(|procedure| procedure.function.name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "update_state",
            "blank_first_line",
            "renamed",
            "state_changed"
        ]
    );

    let update_state = procedure(procedures, "update_state");
    let [(arg_name, arg_type)] = update_state.function.args() else {
        panic!("expected a single argument");
    };
    assert_eq!(arg_name, "new_value");
    // The argument type is a reference that has been registered in `Types`.
    let DataType::Reference(Reference::Named(reference)) = arg_type else {
        panic!("expected a named reference, got {arg_type:?}");
    };
    assert_eq!(
        types.get(reference).map(|named| &*named.name),
        Some("String")
    );
    assert_eq!(update_state.function.result(), None);
    // The leading space is what ends up in the generated JSDoc, so keep it exact.
    assert_eq!(update_state.function.docs(), " Update the state");
    assert!(!update_state.is_event);

    // A leading blank `///` line is dropped rather than becoming a blank JSDoc line.
    assert_eq!(
        procedure(procedures, "blank_first_line").function.docs(),
        " Leading blank doc line"
    );

    let renamed = procedure(procedures, "renamed");
    assert_eq!(
        renamed.function.result(),
        Some(&DataType::Primitive(Primitive::i32))
    );
    // The metadata uses the alias, not the Rust method name.
    assert!(
        procedures
            .iter()
            .all(|procedure| procedure.function.name() != "original")
    );

    assert!(procedure(procedures, "state_changed").is_event);

    assert_eq!(
        args_map[""],
        concat!(
            r#"{"blank_first_line":[],"renamed":["amount"],"state_changed":["new_state"],"#,
            r#""update_state":["new_value"]}"#
        )
    );
}

#[test]
fn renders_procedure_docs_in_the_bindings() {
    let path = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("bindings.ts");
    taurpc::Exporter::new()
        .export(&ApiImpl.into_handler(), &path)
        .unwrap();
    let bindings = std::fs::read_to_string(&path).unwrap();
    std::fs::remove_file(&path).unwrap();

    assert!(bindings.contains("/**  Update the state */"), "{bindings}");
    // A leading blank `///` line doesn't become a blank line in the JSDoc.
    assert!(
        bindings.contains("/**  Leading blank doc line */"),
        "{bindings}"
    );
    assert!(
        bindings.contains("update_state: (newValue: string) => Promise<void>"),
        "{bindings}"
    );
    assert!(
        bindings.contains("renamed: (amount: number) => Promise<number>"),
        "{bindings}"
    );
    assert!(!bindings.contains("original:"), "{bindings}");
}
