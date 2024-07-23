use std::time::Duration;
use serde::Deserialize;

/// `EC2InstanceMetadataIdentityDocument` holds the fetched EC2 instance metadata.
#[derive(Debug, Clone, Deserialize, Default, Eq, PartialEq)]
#[serde(rename_all="camelCase")]
pub struct EC2InstanceIdentityDocument {
    pub private_ip: String,
    pub instance_id: String,
    pub instance_type: String,
    pub account_id: String,
    pub image_id: String,
    pub architecture: String,
    pub region: String,
    pub availability_zone: String,
}

pub (crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    HttpRequestFailed(String),
    Deserialization(String)
}

/// `Client` implements methods to capture EC2 environment metadata information by using the IMDS v2 service.
/// See: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-metadata.html
pub trait Client {
    fn get_instance_identity_document(&self, timeout: Duration) -> Result<EC2InstanceIdentityDocument>;
}

/// `EC2InstanceMetadataClient` implements the `Client` interface to interface with
/// the real EC2 IMDS service.
#[derive(Debug)]
pub (crate) struct EC2InstanceMetadataClient {
    // Base URL where to perform requests. Can be used to mock in unit tests.
    url: String,
    // Internal HTTP client used to perform requests.
    client: reqwest::blocking::Client
}

impl EC2InstanceMetadataClient {
    /// `with_custom_url` initializes an EC2InstanceMetadataClient with the given URL as base.
    /// Could be used in unit tests to mock responses.
    fn with_custom_url(url: String) -> Self {
        Self { url, ..Self::default() }
    }
}

impl Default for EC2InstanceMetadataClient {
    fn default() -> Self {
        Self {
            url: "http://169.254.169.254".to_string(),
            client: reqwest::blocking::Client::default()
        }
    }
}

impl Client for EC2InstanceMetadataClient {
    fn get_instance_identity_document(&self, timeout: Duration) -> Result<EC2InstanceIdentityDocument> {
        let url = format!("{}/latest/dynamic/instance-identity/document", self.url);
        let response = self.client.get(url)
            .timeout(timeout)
            .send()
            .map_err(|e| Error::HttpRequestFailed(format!("HTTP request failed: {:?}", e)))?;

        let document = response
            .json::<EC2InstanceIdentityDocument>()
            .map_err(|e| Error::Deserialization(format!("failed to deserialize document: {:?}", e)))?;

        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use mockito::Server;
    use super::*;

    #[test]
    fn test_get_instance_identity_document() {
        let mut server = Server::new();
        let url = server.url();

        let mock = server.mock("GET", "/latest/dynamic/instance-identity/document")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("{\"accountId\":\"123456789012\",\"architecture\":\"x86_64\",\"availabilityZone\":\"eu-west-1a\",\"billingProducts\":null,\"devpayProductCodes\":null,\"marketplaceProductCodes\":null,\"imageId\":\"ami-5fb8c835\",\"instanceId\":\"i-1234567890abcdef0\",\"instanceType\":\"t2.micro\",\"kernelId\":null,\"pendingTime\":\"2024-07-22T06:33:13Z\",\"privateIp\":\"10.0.0.45\",\"ramdiskId\":null,\"region\":\"eu-west-1\",\"version\":\"2017-09-30\"}")
            .create();

        let expected = EC2InstanceIdentityDocument {
            region: "eu-west-1".to_string(),
            account_id: "123456789012".to_string(),
            architecture: "x86_64".to_string(),
            availability_zone: "eu-west-1a".to_string(),
            image_id: "ami-5fb8c835".to_string(),
            instance_id: "i-1234567890abcdef0".to_string(),
            instance_type: "t2.micro".to_string(),
            private_ip: "10.0.0.45".to_string()
        };

        let client = EC2InstanceMetadataClient::with_custom_url(url);
        let got = client.get_instance_identity_document(Duration::from_secs(10)).unwrap();

        mock.assert();
        assert_eq!(expected, got);
    }

    #[test]
    fn test_get_instance_identity_document_returns_http_error_if_request_fails() {
        let mut server = Server::new();
        let url = server.url();

        let mock = server.mock("GET", "/latest/dynamic/instance-identity/document")
            .with_status(404)
            .create();

        let client = EC2InstanceMetadataClient::with_custom_url(url);
        let got = client.get_instance_identity_document(Duration::from_secs(0));

        assert!(matches!(got, Err(Error::HttpRequestFailed(_))))
    }

    #[test]
    fn test_get_instance_identity_document_returns_deserialization_error_if_document_parsing_fails() {
        let mut server = Server::new();
        let url = server.url();

        let mock = server.mock("GET", "/latest/dynamic/instance-identity/document")
            .with_status(404)
            .create();

        let client = EC2InstanceMetadataClient::with_custom_url(url);
        let got = client.get_instance_identity_document(Duration::from_secs(10));

        assert!(matches!(got, Err(Error::Deserialization(_))))
    }
}
