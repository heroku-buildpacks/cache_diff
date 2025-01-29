use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct AccidentalCustom {
    #[cache_diff(custom = function)]
    i_am_a_custom_field: String,

    normal: String,
}

fn function(_old: &AccidentalCustom, _now: &AccidentalCustom) -> Vec<String> {
    todo!()
}

fn main() {}
