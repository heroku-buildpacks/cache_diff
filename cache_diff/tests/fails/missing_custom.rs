use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct MissingCustom {
    #[cache_diff(ignore = "custom")]
    i_am_a_custom_field: String,

    normal: String,
}

fn main() {}
