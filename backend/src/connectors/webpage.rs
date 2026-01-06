use anyhow::Result;
use async_trait::async_trait;
use html_to_markdown_rs::{ConversionOptions, converter::convert_html};
use reqwest::{Client, header};
use serde_json::Value;

use super::{models::ImportTaskIntermediate, traits::Connector};

#[derive(Debug, Clone)]
pub struct WebpageConnector;

#[async_trait]
impl Connector for WebpageConnector {
    async fn get_intermediate(artifact: Value) -> Result<ImportTaskIntermediate> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "Accept", 
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse()?
        );
        headers.insert("Accept-Language", "en-US,en;q=0.9".parse()?);
        headers.insert("Upgrade-Insecure-Requests", "1".parse()?);

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36") // Spoof a modern Chrome User-Agent on macOS
            .default_headers(headers)
            .cookie_store(true) // Enable cookie jar to handle sessions
            .gzip(true) // Enable automatic decompression
            .redirect(reqwest::redirect::Policy::limited(10)) // Follow redirects
            .build()?;

        let url = artifact.as_str().unwrap();
        let response = client.get(url).send().await?;
        let raw_content = response.text().await?;

        let markdown = convert_html(
            &raw_content,
            &ConversionOptions {
                extract_metadata: false,
                ..Default::default()
            },
        )?;

        Ok(ImportTaskIntermediate {
            title: url.to_string(),
            content: markdown,
        })
    }
}
