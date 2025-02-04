// use serde::Deserialize;
// use specta::{datatype::DataType, Generics, NamedType, TypeCollection};
// use std::str::FromStr;
// use tauri::{
//     ipc::{CommandArg, CommandItem, InvokeError, JavaScriptChannelId},
//     Runtime,
// };

// #[derive(Clone)]
// pub struct Channel<TSend = tauri::ipc::InvokeResponseBody>(tauri::ipc::Channel<TSend>);

// impl<'de, R: Runtime, TSend: Clone> CommandArg<'de, R> for Channel<TSend> {
//     /// Grabs the [`Webview`] from the [`CommandItem`] and returns the associated [`Channel`].
//     fn from_command(command: CommandItem<'de, R>) -> Result<Self, InvokeError> {
//         let name = command.name;
//         let arg = command.key;
//         let webview = command.message.webview();
//         let value: String = Deserialize::deserialize(command)
//             .map_err(|e| tauri::Error::InvalidArgs(name, arg, e))?;
//         Ok(Self(
//             JavaScriptChannelId::from_str(&value)
//                 .map(|id| id.channel_on(webview))
//                 .map_err(|_| {
//                     InvokeError::from(format!(
//                         "invalid channel value `{value}`, expected a string in the `ID` format"
//                     ))
//                 })
//                 .unwrap(),
//         ))
//     }
// }

// // we must manually implement serde::Serialize
// impl<TSend> serde::Serialize for Channel<TSend> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: serde::ser::Serializer,
//     {
//         serializer.serialize_u32(self.0.id())
//     }
// }

// #[derive(specta_macros::Type)]
// struct TauRPCChannel<TSend> {
//     phantom: std::marker::PhantomData<TSend>,
// }

// impl<TSend: specta::Type> specta::Type for Channel<TSend> {
//     fn inline(type_map: &mut TypeCollection, generics: Generics) -> DataType {
//         let generic = TSend::reference(type_map, &[]).inner;
//         let datatype = TauRPCChannel::<TSend>::reference(type_map, &[generic]).inner;
//         type_map.remove(TauRPCChannel::<TSend>::sid());
//         datatype
//     }
// }
