use serde::Serialize;
use std::io;
use ts_rs::TS;

#[derive(TS, Serialize)]
#[ts(export_to = "../../types/")]
struct User {
    user_id: i32,
    first_name: String,
    last_name: String,
}

#[derive(Serialize, TS)]
#[serde(tag = "procedure", content = "data")]
#[ts(export_to = "../../types/index.ts")]
enum ComplexEnum {
    A((String,)),
    B((String, u32)),
    U { name: String },
    V((User,)),
}

pub fn create_example_defs() -> io::Result<()> {
    <User as TS>::export().unwrap();
    <ComplexEnum as TS>::export().unwrap();
    Ok(())
}

trait Api {
    fn test(input1: String, user: User) -> Result<String, ()>;
}

struct ApiImpl;

impl Api for ApiImpl {
    fn test(input1: String, user: User) -> Result<String, ()> {
        Ok(String::from("test"))
    }
}
