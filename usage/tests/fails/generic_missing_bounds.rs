use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Example<T> {
    name: String,
    other: T,
}

fn main() {
    let now = Example::<String> {
        name: "Richard".to_string(),
        other: "John Jacob Jingleheimer Schmidt (his name is my name too)".to_string(),
    };

    let _ = now.diff(&Example::<String> {
        name: "Richard".to_string(),
        other: "schneems".to_string(),
    });
}
