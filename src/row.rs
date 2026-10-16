//! Hand-written additions to [`Row`]. This file is custom code: the generator doesn't produce it.

use serde_json::Value;

use crate::types::Row;

impl Row {
    /// The document's score from the query's `rank_by`, or from `rerank_by` if it's set, which the API returns
    /// in the `$dist` attribute.
    ///
    /// Its meaning depends on the ranking function: for vector search it's the distance from the query vector,
    /// where lower is better, and for BM25 (Best Matching 25, a classic full-text search scoring function, which
    /// scores documents higher when they contain more of the query's terms, especially rare ones) it's the
    /// relevance score, where higher is better. `None` when ordering by an attribute.
    pub fn dist(&self) -> Option<f64> {
        self.get("$dist").and_then(Value::as_f64)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn reads_the_score() {
        let row: Row = serde_json::from_value(json!({ "id": 1, "$dist": 0.5 })).unwrap();
        assert_eq!(row.dist(), Some(0.5));

        let row: Row = serde_json::from_value(json!({ "id": 1 })).unwrap();
        assert_eq!(row.dist(), None);
    }
}
