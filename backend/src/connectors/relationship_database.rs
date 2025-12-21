use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{mysql::MySqlPool, postgres::PgPool, sqlite::SqlitePool, Row};

use super::{models::ImportTaskIntermediate, traits::Connector};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelationshipDatabaseType {
    MySQL,
    Postgres,
    SQLite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDatabaseArtifact {
    pub database_type: RelationshipDatabaseType,
    pub username: String,
    pub password: String,
    pub host: String,
    pub port: String,
    pub database_name: Option<String>,
    pub query: String,
    pub column_to_fetch: String, // this will become the `content`
    pub table_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RelationshipDatabaseConnector;

#[async_trait]
impl Connector for RelationshipDatabaseConnector {
    async fn get_intermediate(artifact: Value) -> Result<ImportTaskIntermediate> {
        let relationship_database_artifact: RelationshipDatabaseArtifact = serde_json::from_value(artifact)?;
        
        let mut content = String::new();
        let query = &relationship_database_artifact.query;
        let column = &relationship_database_artifact.column_to_fetch;

        match relationship_database_artifact.database_type {
            RelationshipDatabaseType::MySQL => {
                let mut connection_string = format!(
                    "mysql://{}:{}@{}:{}/{}",
                    relationship_database_artifact.username,
                    relationship_database_artifact.password,
                    relationship_database_artifact.host,
                    relationship_database_artifact.port,
                    relationship_database_artifact.database_name.as_deref().unwrap_or(""),
                );
                if connection_string.ends_with('/') {
                    connection_string.pop();
                }

                let pool = MySqlPool::connect(&connection_string).await
                    .context("Failed to connect to MySQL database")?;

                let rows = sqlx::query(query)
                    .fetch_all(&pool)
                    .await?;
                
                for row in rows {
                    if let Ok(val) = row.try_get::<String, _>(column.as_str()) {
                        if !content.is_empty() {
                            content.push('\n');
                        }
                        content.push_str(&val);
                    }
                }
            },
            RelationshipDatabaseType::Postgres => {
                let mut connection_string = format!(
                    "postgres://{}:{}@{}:{}/{}",
                    relationship_database_artifact.username,
                    relationship_database_artifact.password,
                    relationship_database_artifact.host,
                    relationship_database_artifact.port,
                    relationship_database_artifact.database_name.as_deref().unwrap_or("postgres"),
                );
                if connection_string.ends_with('/') {
                    connection_string.pop();
                }

                let pool = PgPool::connect(&connection_string).await
                    .context("Failed to connect to Postgres database")?;

                let rows = sqlx::query(query)
                    .fetch_all(&pool)
                    .await?;
                
                for row in rows {
                    if let Ok(val) = row.try_get::<String, _>(column.as_str()) {
                        if !content.is_empty() {
                            content.push('\n');
                        }
                        content.push_str(&val);
                    }
                }
            },
            RelationshipDatabaseType::SQLite => {
                // For SQLite, host is treated as the file path
                let connection_string = format!("sqlite://{}", relationship_database_artifact.host);
                
                let pool = SqlitePool::connect(&connection_string).await
                    .context("Failed to connect to SQLite database")?;

                let rows = sqlx::query(query)
                    .fetch_all(&pool)
                    .await?;
                
                for row in rows {
                    if let Ok(val) = row.try_get::<String, _>(column.as_str()) {
                        if !content.is_empty() {
                            content.push('\n');
                        }
                        content.push_str(&val);
                    }
                }
            },
        }
            
        Ok(
            ImportTaskIntermediate {
                title: relationship_database_artifact.table_name.unwrap_or_else(|| "Query Result".to_string()),
                content, 
            }
        )
    }
}
