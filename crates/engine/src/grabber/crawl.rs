use super::{document, *};
use reqwest::{
    header::{ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, CONTENT_TYPE, LOCATION},
    Client, Response,
};
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use url::Url;

#[derive(Clone)]
pub(crate) struct Network {
    client: Client,
    starts: Arc<Mutex<BTreeMap<String, tokio::time::Instant>>>,
    robots: Arc<Mutex<BTreeMap<String, Result<String, String>>>>,
}
pub(crate) struct Fetched {
    pub url: String,
    pub mime: String,
    pub size: Option<u64>,
    pub kind: ItemKind,
    pub original: Option<Vec<u8>>,
    pub links: Vec<document::Link>,
}
#[derive(Debug)]
pub(crate) struct CrawlError {
    pub message: String,
    pub blocked: bool,
    pub limit: bool,
}
impl From<String> for CrawlError {
    fn from(message: String) -> Self {
        Self {
            message,
            blocked: false,
            limit: false,
        }
    }
}
impl Network {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("RustySiteGrabber/0.2.1")
            .redirect(reqwest::redirect::Policy::none())
            .referer(false)
            .retry(reqwest::retry::never())
            .connect_timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| "Cannot initialize crawler HTTP client.")?;
        Ok(Self {
            client,
            starts: Default::default(),
            robots: Default::default(),
        })
    }
    pub async fn clear_failures(&self) {
        self.robots.lock().await.retain(|_, rules| rules.is_ok());
    }
    pub async fn blocked(&self, url: &str) -> bool {
        let Ok(url) = Url::parse(url) else {
            return true;
        };
        self.robots
            .lock()
            .await
            .get(&url.origin().ascii_serialization())
            .is_some_and(|r| r.is_err())
    }
    async fn gate(&self, url: &Url, delay: u64) {
        let host = url.host_str().unwrap_or_default().to_owned();
        let deadline = {
            let mut starts = self.starts.lock().await;
            let now = tokio::time::Instant::now();
            let deadline = starts.get(&host).copied().unwrap_or(now).max(now);
            starts.insert(host, deadline + Duration::from_millis(delay));
            deadline
        };
        tokio::time::sleep_until(deadline).await;
    }
    async fn send(&self, url: &Url, config: &ProjectConfig) -> Result<Response, String> {
        self.gate(url, config.host_delay_ms).await;
        tokio::time::timeout(
            Duration::from_secs(30),
            self.client
                .get(url.clone())
                .header(ACCEPT_ENCODING, "identity")
                .send(),
        )
        .await
        .map_err(|_| "The crawler connection timed out.".to_string())?
        .map_err(|_| "The crawler connection failed. Check network and TLS certificates.".into())
    }
    async fn body(&self, mut response: Response, limit: u64) -> Result<Vec<u8>, CrawlError> {
        if response
            .headers()
            .get(CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| !v.eq_ignore_ascii_case("identity"))
        {
            return Err(
                "The server ignored identity encoding; this document was not parsed."
                    .to_owned()
                    .into(),
            );
        }
        if response.content_length().is_some_and(|n| n > limit) {
            return Err(CrawlError {
                message:
                    "Document size limit reached. Increase the project limit or narrow its scope."
                        .into(),
                blocked: false,
                limit: true,
            });
        }
        let mut bytes = Vec::new();
        loop {
            let chunk = tokio::time::timeout(Duration::from_secs(30), response.chunk())
                .await
                .map_err(|_| "The crawler response made no progress for 30 seconds.".to_string())?
                .map_err(|_| "The crawler response was interrupted.".to_string())?;
            let Some(chunk) = chunk else {
                break;
            };
            if bytes.len() as u64 + chunk.len() as u64 > limit {
                return Err(CrawlError{message:"Document size limit reached. Increase the project limit or narrow its scope.".into(),blocked:false,limit:true});
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    async fn check_robots(&self, url: &Url, config: &ProjectConfig) -> Result<(), CrawlError> {
        if !config.respect_robots {
            return Ok(());
        }
        let origin = url.origin().ascii_serialization();
        let cached = { self.robots.lock().await.get(&origin).cloned() };
        let rules = if let Some(cached) = cached {
            cached
        } else {
            let result=async {
                let mut robots=Url::parse(&format!("{origin}/robots.txt")).map_err(|_|"Invalid robots origin.".to_owned())?;
                for hop in 0..=config.max_redirects{
                    if !(host_allowed(&robots,&config.page_hosts)||host_allowed(&robots,&config.asset_hosts)){return Err("Robots redirect left the allowed hosts.".into());}
                    let response=self.send(&robots,config).await?;
                    if response.status().is_redirection(){
                        if hop==config.max_redirects{return Err("Robots redirect limit reached.".into());}
                        let target=response.headers().get(LOCATION).and_then(|v|v.to_str().ok()).ok_or("Invalid robots redirect.")?;
                        robots=robots.join(target).map_err(|_|"Invalid robots redirect.")?;continue;
                    }
                    if response.status().is_client_error(){return Ok(String::new());}
                    if !response.status().is_success(){return Err("Robots rules are temporarily unavailable. Resume to retry this origin.".into());}
                    // RFC 9309 permits a parser limit of at least 500 KiB.
                    let bytes=self.body(response,512*1024).await.map_err(|e|e.message)?;
                    return Ok(String::from_utf8_lossy(&bytes).into_owned());
                }
                Err("Robots redirect limit reached.".into())
            }.await;
            self.robots
                .lock()
                .await
                .insert(origin.clone(), result.clone());
            result
        };
        match &rules {
            Ok(rules) => {
                if robotstxt::DefaultMatcher::default().one_agent_allowed_by_robots(
                    rules,
                    "RustySiteGrabber",
                    url.as_str(),
                ) {
                    Ok(())
                } else {
                    Err("Blocked by this origin's robots.txt rules."
                        .to_owned()
                        .into())
                }
            }
            Err(message) => Err(CrawlError {
                message: message.clone(),
                blocked: true,
                limit: false,
            }),
        }
    }
    pub async fn fetch(
        &self,
        item: GrabberItem,
        config: ProjectConfig,
    ) -> Result<Fetched, CrawlError> {
        let mut url = public_url(&item.url)?;
        for hop in 0..=config.max_redirects {
            if !(config.allows(&url, true) || config.allows(&url, false)) {
                return Err("URL or redirect is outside the allowed project scope."
                    .to_owned()
                    .into());
            }
            self.check_robots(&url, &config).await?;
            let response = self.send(&url, &config).await?;
            if response.status().is_redirection() {
                if hop == config.max_redirects {
                    return Err("Crawler redirect limit reached.".to_owned().into());
                }
                let value = response
                    .headers()
                    .get(LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| "Invalid redirect location.".to_owned())?;
                url = public_url(
                    response
                        .url()
                        .join(value)
                        .map_err(|_| "Invalid redirect location.".to_owned())?
                        .as_str(),
                )?;
                continue;
            }
            if !response.status().is_success() {
                return Err(format!("Crawler HTTP {}.", response.status().as_u16()).into());
            }
            let content_type = response
                .headers()
                .get(CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_owned();
            let mime = content_type
                .split(';')
                .next()
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase();
            let size = response
                .headers()
                .get(CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok());
            let kind = match mime.as_str() {
                "text/html" | "application/xhtml+xml" => ItemKind::Page,
                "text/css" => ItemKind::Stylesheet,
                _ => ItemKind::Asset,
            };
            if !config.allows(&url, kind == ItemKind::Page) {
                return Err(
                    "The final resource type is outside its page/asset allowlist."
                        .to_owned()
                        .into(),
                );
            }
            let (original, links) = if matches!(kind, ItemKind::Page | ItemKind::Stylesheet)
                && !(kind == ItemKind::Page && item.depth > config.max_depth)
            {
                let bytes = self.body(response, config.document_limit).await?;
                let input = document::decode(&bytes, &content_type);
                if input.len() as u64 > config.document_limit {
                    return Err(CrawlError {
                        message: "Decoded document size limit reached. Rescan with a larger limit."
                            .into(),
                        blocked: false,
                        limit: true,
                    });
                }
                let base = url.clone();
                let limit = config.max_results;
                let links = tokio::task::spawn_blocking(move || {
                    document::discover(&input, &base, kind, limit)
                })
                .await
                .map_err(|_| "Document parsing worker failed.".to_owned())?
                .map_err(|error| match error {
                    document::DiscoveryError::Limit => CrawlError {
                        message:
                            "Document link count limit reached. Rescan with a higher result limit."
                                .into(),
                        blocked: false,
                        limit: true,
                    },
                    document::DiscoveryError::Parse(message) => message.into(),
                })?;
                (
                    Some(document::decode(&bytes, &content_type).into_bytes()),
                    links,
                )
            } else {
                (None, vec![])
            };
            return Ok(Fetched {
                url: url.into(),
                mime,
                size,
                kind,
                original,
                links,
            });
        }
        Err("Crawler redirect limit reached.".to_owned().into())
    }
}

/// Applied by both ordinary and segmented clients only for grabber members.
pub(crate) fn redirect_policy(member: Option<&ProjectMember>) -> reqwest::redirect::Policy {
    match member {
        None => reqwest::redirect::Policy::limited(10),
        Some(member) => {
            let hosts = member.allowed_hosts.clone();
            let limit = member.max_redirects;
            let excluded = member.exclude_urls.clone();
            reqwest::redirect::Policy::custom(move |attempt| {
                if attempt.previous().len() > limit {
                    attempt.error("Site Grabber redirect limit reached.")
                } else if !host_allowed(attempt.url(), &hosts)
                    || excluded
                        .iter()
                        .any(|p| super::model::glob(p, attempt.url().as_str()))
                {
                    attempt.error("Site Grabber redirect left the allowed project scope.")
                } else {
                    attempt.follow()
                }
            })
        }
    }
}
