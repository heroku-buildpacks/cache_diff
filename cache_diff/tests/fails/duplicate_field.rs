use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct CustomDiffFn {
    #[cache_diff(rename = "foo", rename = "bar")]
    name: String,
}

fn main() {}
