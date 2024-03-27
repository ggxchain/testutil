#[cfg_attr(
    feature = "brooklyn",
    subxt::subxt(
        runtime_metadata_path = "./src/metadata/metadata_ggx_brooklyn.scale",
        derive_for_all_types = "Clone",
        substitute_type(
            path = "bitcoin::address::Address",
            with = "::subxt::utils::Static<bitcoin::Address>"
        )
    )
)]
#[cfg_attr(
    feature = "sydney",
    subxt::subxt(
        runtime_metadata_path = "./src/metadata/metadata_ggx_sydney.scale",
        derive_for_all_types = "Clone",
        substitute_type(
            path = "bitcoin::address::Address",
            with = "::subxt::utils::Static<bitcoin::Address>"
        )
    )
)]
pub mod ggx {}
