use axum::Json;
use rstest::*;
use serde_json::json;

use sample_graph_api::*;

#[rstest]
async fn test_version() {
    let result = version().await.unwrap();
    assert!(matches!(result, Json(..)));
    assert_eq!(result.0, json!(0));
}
