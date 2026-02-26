# delete_collection -> delete_collections 

Previous request

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteCollectionRequest {
    pub collection_metadata_ids: Vec<String>,
}
```

Now
```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DeleteCollectionRequest {
    pub username: String,
    pub collection_metadata_ids: Vec<String>,
}
```