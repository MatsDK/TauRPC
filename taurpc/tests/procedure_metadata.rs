//! The procedure metadata is built by the `#[taurpc::procedures]` macro, so this
//! makes sure it matches what the exporter used to get out of `specta::function`.

use taurpc::{Exportable, TauRpcFunction};

#[taurpc::procedures]
trait Api {
    /// Update the state
    async fn update_state(new_value: String);

    #[taurpc(alias = "renamed")]
    fn renamed(amount: i32) -> i32;

    #[taurpc(event)]
    async fn state_changed(new_state: String);
}

#[derive(Clone)]
struct ApiImpl;

#[taurpc::resolvers]
impl Api for ApiImpl {
    async fn update_state(self, _new_value: String) {}

    fn renamed(self, amount: i32) -> i32 {
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
    let (mut types, procedures, args_map) = ApiImpl.into_handler().generate_types();
    let procedures = &procedures[""];

    let names = procedures
        .iter()
        .map(|procedure| procedure.function.name())
        .collect::<Vec<_>>();
    assert_eq!(names, ["update_state", "renamed", "state_changed"]);

    let update_state = procedure(procedures, "update_state");
    assert_eq!(
        update_state
            .function
            .args()
            .iter()
            .map(|(name, _)| name.as_ref())
            .collect::<Vec<_>>(),
        ["new_value"]
    );
    assert_eq!(update_state.function.result(), None);
    assert!(update_state.function.docs().contains("Update the state"));
    assert!(!update_state.is_event);

    let renamed = procedure(procedures, "renamed");
    assert_eq!(
        renamed.function.result(),
        Some(&<i32 as taurpc::specta::Type>::definition(&mut types))
    );

    assert!(procedure(procedures, "state_changed").is_event);

    assert_eq!(
        args_map[""],
        r#"{"renamed":["amount"],"state_changed":["new_state"],"update_state":["new_value"]}"#
    );
}
