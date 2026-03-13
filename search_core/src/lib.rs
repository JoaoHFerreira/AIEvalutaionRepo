use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SearchResult {
    pub id: u32,
    pub title: String,
    pub description: String,
}

pub fn filter_results(data: Vec<SearchResult>, query: &str) -> Vec<SearchResult> {
    let query = query.to_lowercase();
    data.into_iter()
        .filter(|item| {
            item.title.to_lowercase().contains(&query)
                || item.description.to_lowercase().contains(&query)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rstest::rstest;

    #[test]
    fn test_serialization() {
        let result = SearchResult {
            id: 1,
            title: "Test".to_string(),
            description: "Desc".to_string(),
        };
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: SearchResult = serde_json::from_str(&serialized).unwrap();
        assert_eq!(result, deserialized);
    }

    proptest! {
        #[test]
        fn test_serialization_roundtrip(id in 0u32..10000u32, title in ".*", description in ".*") {
            let result = SearchResult { id, title: title.clone(), description: description.clone() };
            let serialized = serde_json::to_string(&result).unwrap();
            let deserialized: SearchResult = serde_json::from_str(&serialized).unwrap();
            prop_assert_eq!(result, deserialized);
        }
    }

    fn mock_data() -> Vec<SearchResult> {
        vec![
            SearchResult {
                id: 1,
                title: "Rust Programming".to_string(),
                description: "Reliable and efficient software.".to_string(),
            },
            SearchResult {
                id: 2,
                title: "WASM in Action".to_string(),
                description: "WebAssembly is a binary format.".to_string(),
            },
        ]
    }

    #[rstest]
    #[case("", 2)] // Empty query matches all
    #[case("rust", 1)] // Title match
    #[case("Reliable", 1)] // Description match
    #[case("RUST", 1)] // Case-insensitive title
    #[case("RELIABLE", 1)] // Case-insensitive description
    #[case("action", 1)] // Another title match
    #[case("nomatch", 0)] // No matches
    #[case("in", 2)] // Substring match (matches "Programming" and "in")
    fn test_filter_results(#[case] query: &str, #[case] expected_count: usize) {
        let data = mock_data();
        let results = filter_results(data, query);
        assert_eq!(results.len(), expected_count);
    }
}
