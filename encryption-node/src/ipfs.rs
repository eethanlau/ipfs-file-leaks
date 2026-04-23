//! HTTP client for a Kubo-compatible IPFS node.

use reqwest::multipart;
use url::Url;

const ADD_PATH: &str = "api/v0/add";
const CAT_PATH: &str = "api/v0/cat";

pub struct IpfsClient {
    http: reqwest::Client,
    base: Url,
}

impl IpfsClient {
    /// `base` must have a trailing slash; relative paths are joined onto it.
    pub fn new(base: Url) -> Self {
        Self {
            http: reqwest::Client::new(),
            base,
        }
    }

    pub async fn add(&self, data: Vec<u8>) -> Result<String, IpfsError> {
        let url = self.base.join(ADD_PATH).map_err(IpfsError::Url)?;
        let part = multipart::Part::bytes(data).file_name("file");
        let form = multipart::Form::new().part("file", part);

        let response = self.http.post(url).multipart(form).send().await?;
        let json: serde_json::Value = response.json().await?;

        json["Hash"]
            .as_str()
            .map(str::to_string)
            .ok_or(IpfsError::MissingHash)
    }

    pub async fn cat(&self, cid: &str) -> Result<Vec<u8>, IpfsError> {
        let mut url = self.base.join(CAT_PATH).map_err(IpfsError::Url)?;
        url.query_pairs_mut().append_pair("arg", cid);

        let response = self.http.post(url).send().await?;
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum IpfsError {
    #[error("IPFS HTTP transport error")]
    Transport(#[from] reqwest::Error),
    #[error("failed to construct IPFS URL")]
    Url(#[source] url::ParseError),
    #[error("IPFS add response missing Hash field")]
    MissingHash,
}
