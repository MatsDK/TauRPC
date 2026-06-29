use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::ipc::Channel;

#[derive(Debug, Serialize, Deserialize, Type)]
pub enum CustomError {
    SimpleError,
    MessageError(String),
    ComplexError { code: i32, details: String },
}

#[derive(Debug, Serialize, Deserialize, Type)]
pub struct ComplexData {
    pub id: u32,
    pub payload: String,
}

// Another error type to test multiple error types
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct StructError {
    pub status: u16,
    pub message: String,
}

#[taurpc::procedures(path = "error_testing")]
pub trait ErrorTestingApi {
    // Basic string error
    async fn test_string_error(fail: bool) -> Result<String, String>;

    // Enum error
    async fn test_enum_error(fail: bool) -> Result<ComplexData, CustomError>;

    // Struct error
    async fn test_struct_error(fail: bool) -> Result<(), StructError>;

    // Result with a channel argument (might break or have weird type phases)
    async fn test_with_channel(fail: bool, on_update: Channel<String>) -> Result<(), CustomError>;
}

#[derive(Clone)]
pub struct ErrorTestingApiImpl;

#[taurpc::resolvers]
impl ErrorTestingApi for ErrorTestingApiImpl {
    async fn test_string_error(self, fail: bool) -> Result<String, String> {
        if fail {
            Err("This is a basic string error!".to_string())
        } else {
            Ok("Success!".to_string())
        }
    }

    async fn test_enum_error(self, fail: bool) -> Result<ComplexData, CustomError> {
        if fail {
            Err(CustomError::ComplexError {
                code: 500,
                details: "Database connection failed".to_string(),
            })
        } else {
            Ok(ComplexData {
                id: 42,
                payload: "Some data".to_string(),
            })
        }
    }

    async fn test_struct_error(self, fail: bool) -> Result<(), StructError> {
        if fail {
            Err(StructError {
                status: 404,
                message: "Not found".to_string(),
            })
        } else {
            Ok(())
        }
    }

    async fn test_with_channel(self, fail: bool, on_update: Channel<String>) -> Result<(), CustomError> {
        let _ = on_update.send("Starting process...".to_string());
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        if fail {
            let _ = on_update.send("Failing!".to_string());
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            Err(CustomError::MessageError("Failed during process".to_string()))
        } else {
            let _ = on_update.send("Finished successfully!".to_string());
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            Ok(())
        }
    }
}
