// They're tests: who cares if we use "foo"
#[allow(clippy::disallowed_names)]
// We don't care about test performance so much and makes for better diffs
#[allow(clippy::redundant_clone)]
mod macros;
