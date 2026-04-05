use near_sdk::near;

/// Top-level contract metadata.
#[derive(PartialEq, Eq, Clone, Debug)]
#[near(serializers = [borsh, json])]
pub struct ContractMetadata {
    /// Specification version. For example: `"mt-1.0.0"`
    pub spec: String,
    /// Contract name. For example: `"Zoink's Digitial Sword Collection"`
    pub name: String,
}

impl ContractMetadata {
    /// The specification version implemented by this library.
    ///
    /// This version is specified here: <https://github.com/near/NEPs/blob/8dc84d48129e65da51a6ab5a6f7d70f6a61b9f27/neps/nep-0245/Metadata.md>
    pub const SPEC: &'static str = "mt-1.0.0";

    /// Constructs an instance of `ContractMetadata` with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            spec: Self::SPEC.to_string(),
            name: name.into(),
        }
    }
}
