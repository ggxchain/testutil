// re-export publicly
pub use testcontainers::ContainerAsync;

#[cfg(feature = "brooklyn")]
pub mod metadata;

pub mod containers;

/// in case of subxt error, panic with a meaningful message
pub fn handle_tx_error(e: subxt::Error) -> ! {
    match e {
        subxt::Error::Runtime(subxt::error::DispatchError::Module(error)) => {
            let details = error.details().expect("cannot get details");
            let pallet = details.pallet.name();
            let error = &details.variant;
            panic!("Extrinsic failed with an error: {pallet}::{error:?}")
        }
        _ => {
            panic!("Extrinsic failed with an error: {}", e)
        }
    };
}

/// macro vecs! which creates a Vec<String> from &str:
/// ```
/// use testutil::vecs;
/// let v: Vec<String> = vecs!["a", "b"];
/// ```
#[macro_export]
macro_rules! vecs {
    ($($x:expr),*) => {{
        let mut v = Vec::new();
        $(
            v.push($x.to_string());
        )*
        v
    }};
}
