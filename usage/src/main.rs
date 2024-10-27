use cache_diff::CacheDiff;

#[derive(CacheDiff)]
struct Hello {
    name: String,
}
fn main() {
    let _ = Hello {
        name: "world".to_string(),
    };
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn is_diff<T: cache_diff::CacheDiff>(_in: &T) {}

    #[test]
    fn ignore_a_field() {
        #[derive(CacheDiff)]
        struct Metadata {
            ruby_version: String,
            #[cache_diff(ignore)]
            #[allow(dead_code)]
            _modified_by: String,
        }

        let metadata = Metadata {
            ruby_version: "3.4.0".to_string(),
            _modified_by: "richard".to_string(),
        };

        let diff = metadata.diff(&Metadata {
            ruby_version: "3.3.0".to_string(),
            _modified_by: "not rich".to_string(),
        });
        assert_eq!(diff.len(), 1);
        let contents = diff.join(" ");
        assert!(
            !contents.contains("modified"),
            "Unexpected contents {contents}"
        );
    }

    #[test]
    fn auto_display_path_buff() {
        #[derive(CacheDiff)]
        struct Metadata {
            path: PathBuf,
        }
        let metadata = Metadata {
            path: PathBuf::from("/tmp"),
        };
        let diff = metadata.diff(&Metadata {
            path: PathBuf::from("/tmp2"),
        });

        assert_eq!(diff.len(), 1);
        let contents = diff.join(" ");
        assert!(
            contents.contains("/tmp"),
            "Unexpected contents '{contents}'"
        );
    }

    #[test]
    fn ignore_rename_display_field() {
        fn my_display(value: &String) -> String {
            format!("custom {value}")
        }
        #[derive(CacheDiff)]
        struct Metadata {
            #[cache_diff(rename="Ruby version", display=my_display)]
            version: String,
        }
        let metadata = Metadata {
            version: "3.4.0".to_string(),
        };
        let diff = metadata.diff(&Metadata {
            version: "3.3.0".to_string(),
        });

        assert_eq!(diff.len(), 1);
        let contents = diff.join(" ");
        assert!(
            contents.contains("custom 3.4.0"),
            "Expected `{contents}` to contain 'custom 3.4.0'"
        );
    }

    #[test]
    fn ignore_rename_field() {
        #[derive(CacheDiff)]
        struct Metadata {
            #[cache_diff(rename = "Ruby version")]
            version: String,
        }
        let metadata = Metadata {
            version: "3.4.0".to_string(),
        };
        let diff = metadata.diff(&Metadata {
            version: "3.3.0".to_string(),
        });

        assert_eq!(diff.len(), 1);
        let contents = diff.join(" ");
        assert!(
            contents.contains("Ruby version"),
            "Expected `{contents}` to contain Ruby version"
        );
    }

    // #[test]
    // fn ignore_field() {
    //     #[derive(CacheDiff)]
    //     struct Metadata {
    //         ruby_version: String,
    //         #[cache_diff(ignore)]
    //         modified_by: String,
    //     }
    //     let metadata = Metadata {
    //         ruby_version: "3.4.0".to_string(),
    //         modified_by: "richard".to_string(),
    //     };
    //     let diff = metadata.diff(&Metadata {
    //         ruby_version: "3.3.0".to_string(),
    //         modified_by: "not rich".to_string(),
    //     });

    //     assert_eq!(diff.len(), 1);
    // }

    #[test]
    fn test_replace_space() {
        #[derive(CacheDiff)]
        struct Metadata {
            ruby_version: String,
        }
        let metadata = Metadata {
            ruby_version: "3.4.0".to_string(),
        };
        let diff = metadata.diff(&Metadata {
            ruby_version: "3.3.0".to_string(),
        });
        assert_eq!(diff.len(), 1);
        assert!(diff.join(" ").contains("ruby version"));
    }

    #[test]
    fn test_cache_diff() {
        #[derive(CacheDiff)]
        struct Person {
            _name: String,
        }
        let richard = Person {
            _name: "richard".to_string(),
        };
        is_diff(&richard);
        let diff = richard.diff(&Person {
            _name: "rich".to_string(),
        });

        assert_eq!(diff.len(), 1);
    }
}
