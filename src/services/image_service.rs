use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use tracing::{info, warn};

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PexelsSearchResponse {
    total_results: i32,
    page: i32,
    per_page: i32,
    photos: Vec<PexelsImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PexelsImage {
    id: i64,
    width: i32,
    height: i32,
    url: String,
    photographer: String,
    photographer_url: String,
    src: PexelsImageSrc,
    alt: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct PexelsImageSrc {
    original: String,
    large2x: String,
    large: String,
    medium: String,
    small: String,
    portrait: String,
    landscape: String,
    tiny: String,
}

pub struct ImageService {
    client: Client,
    pexels_api_key: String,
}

impl ImageService {
    pub fn new() -> Result<Self> {
        let pexels_api_key = std::env::var("PEXELS_API_KEY").unwrap_or_else(|_| String::new());

        if pexels_api_key.is_empty() {
            warn!("PEXELS_API_KEY not set, will use fallback images only");
        }

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
            pexels_api_key,
        })
    }

    pub async fn generate_market_image_with_fallback(&self, question: &str) -> String {
        let start_time = std::time::Instant::now();

        info!(
            "Attempting to generate image for question: \"{}...\"",
            &question[..question.len().min(100)]
        );

        if !self.pexels_api_key.is_empty() {
            match self.generate_market_image(question).await {
                Ok(Some(image_url)) => {
                    let duration = start_time.elapsed();
                    info!("Successfully generated Pexels image in {:?}", duration);
                    return image_url;
                }
                Ok(None) => {
                    info!("No image found from Pexels API, using fallback");
                }
                Err(e) => {
                    warn!("Pexels API error: {}, using fallback", e);
                }
            }
        }

        let fallback_image = Self::get_fallback_image(question);
        let duration = start_time.elapsed();
        info!("Fallback image selected in {:?}", duration);

        fallback_image
    }

    async fn generate_market_image(&self, question: &str) -> Result<Option<String>> {
        let clean_query = Self::clean_question_for_search(question);

        let response = self
            .client
            .get("https://api.pexels.com/v1/search")
            .header("Authorization", &self.pexels_api_key)
            .query(&[
                ("query", clean_query.as_str()),
                ("per_page", "1"),
                ("orientation", "square"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let _body = response.text().await.unwrap_or_default();

            if status == 429 {
                warn!("Pexels API rate limit exceeded");
            } else if status == 403 {
                warn!("Pexels API quota exceeded or forbidden");
            }

            return Ok(None);
        }

        let data: PexelsSearchResponse = response.json().await?;

        if let Some(photo) = data.photos.first() {
            return Ok(Some(photo.src.medium.clone()));
        }

        Ok(None)
    }

    fn clean_question_for_search(question: &str) -> String {
        let mut clean_query = question
            .trim()
            .trim_start_matches(|c: char| {
                matches!(
                    c.to_lowercase().collect::<String>().as_str(),
                    "will"
                        | "who"
                        | "what"
                        | "when"
                        | "where"
                        | "why"
                        | "how"
                        | "is"
                        | "are"
                        | "does"
                        | "do"
                        | "did"
                )
            })
            .trim_end_matches('?')
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        if clean_query.len() > 100 {
            clean_query = clean_query[..100].to_string();
        }

        clean_query
    }

    pub fn get_fallback_image(question: &str) -> String {
        let lower_question = question.to_lowercase();

        if lower_question.contains("election")
            || lower_question.contains("president")
            || lower_question.contains("vote")
            || lower_question.contains("politic")
            || lower_question.contains("campaign")
        {
            return "https://images.pexels.com/photos/1550337/pexels-photo-1550337.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        if lower_question.contains("sport")
            || lower_question.contains("game")
            || lower_question.contains("championship")
            || lower_question.contains("match")
            || lower_question.contains("team")
            || lower_question.contains("player")
        {
            return "https://images.pexels.com/photos/274422/pexels-photo-274422.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        if lower_question.contains("stock")
            || lower_question.contains("market")
            || lower_question.contains("price")
            || lower_question.contains("finance")
            || lower_question.contains("trading")
            || lower_question.contains("economy")
        {
            return "https://images.pexels.com/photos/730547/pexels-photo-730547.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        if lower_question.contains("weather")
            || lower_question.contains("temperature")
            || lower_question.contains("rain")
            || lower_question.contains("storm")
            || lower_question.contains("climate")
            || lower_question.contains("hurricane")
        {
            return "https://images.pexels.com/photos/1154510/pexels-photo-1154510.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        if lower_question.contains("tech")
            || lower_question.contains("ai")
            || lower_question.contains("crypto")
            || lower_question.contains("bitcoin")
            || lower_question.contains("blockchain")
            || lower_question.contains("computer")
        {
            return "https://images.pexels.com/photos/1181671/pexels-photo-1181671.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        if lower_question.contains("movie")
            || lower_question.contains("film")
            || lower_question.contains("actor")
            || lower_question.contains("oscar")
            || lower_question.contains("entertainment")
        {
            return "https://images.pexels.com/photos/7991579/pexels-photo-7991579.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        if lower_question.contains("science")
            || lower_question.contains("research")
            || lower_question.contains("study")
            || lower_question.contains("experiment")
        {
            return "https://images.pexels.com/photos/2280549/pexels-photo-2280549.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string();
        }

        "https://images.pexels.com/photos/1323550/pexels-photo-1323550.jpeg?auto=compress&cs=tinysrgb&w=400&h=400&fit=crop".to_string()
    }
}
