use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Example<T>
where
    T: std::fmt::Display + Eq,
{
    name: String,
    other: T,
}

#[derive(CacheDiff)]
struct ExampleToo<T, V>
where
    T: std::fmt::Display + Eq,
    V: std::fmt::Display + PartialEq,
{
    one: T,
    two: V,
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

    let new = ExampleToo::<String, String> {
        one: "One".to_string(),
        two: "Two".to_string(),
    };

    let _ = new.diff(&ExampleToo::<String, String> {
        one: "Won".to_string(),
        two: "Too".to_string(),
    });
}
