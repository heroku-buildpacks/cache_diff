use cache_diff::CacheDiff;
use serde::Deserialize;

#[derive(Debug, Deserialize, CacheDiff)]
#[cache_diff(custom = custom_diff)]
#[serde(deny_unknown_fields)]
struct Example {
    name: String,
}

fn custom_diff(_old: &Example, _now: &Example) -> Vec<String> {
    todo!()
}

fn main() {}
