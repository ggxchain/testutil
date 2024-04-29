use testcontainers::Container;

use testcontainers::core::{Image, WaitFor};
use testcontainers::ImageArgs;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct HermesImage {}
//
// impl Image for HermesImage {
//     type Args = HermesArgs;
//
//     fn name(&self) -> String {
//         "ggxdocker/hermes".to_string()
//     }
//
//     fn tag(&self) -> String {
//         "v1".to_string()
//     }
//
//     fn ready_conditions(&self) -> Vec<WaitFor> {
//         vec![]
//     }
// }
