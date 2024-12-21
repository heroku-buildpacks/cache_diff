// Ensure multiple derives can be in the same file
#[derive(CacheDiff)]
struct Dog {
    woof: String,
}
#[allow(dead_code)]
#[derive(CacheDiff)]
struct Cat {
    meow: String,
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_cat() {
        let diff = Cat {
            meow: "Meow".to_string(),
        }
        .diff(&Cat {
            meow: "Woem".to_string(),
        });
        assert!(diff.len() == 1);
    }
    #[test]
    fn test_dog() {
        let diff = Dog {
            woof: "Woof".to_string(),
        }
        .diff(&Dog {
            woof: "Foow".to_string(),
        });
        assert!(diff.len() == 1);
    }
}
