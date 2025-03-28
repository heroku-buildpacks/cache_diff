use cache_diff::CacheDiff;
#[derive(PartialEq)]
struct NotDisplay;

#[derive(CacheDiff)]
struct Example {
    field: NotDisplay,
}

fn main() {}
