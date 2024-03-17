#[cfg(feature = "near-sdk-4")]
extern crate near_sdk_4 as near_sdk;

#[cfg(feature = "near-sdk-5")]
extern crate near_sdk_5 as near_sdk;

// They're tests: who cares if we use "foo"
#[allow(clippy::disallowed_names)]
// We don't care about test performance so much and makes for better diffs
#[allow(clippy::redundant_clone)]
mod macros;
