mod instance_metadata;

use std::time::Duration;
use opentelemetry::KeyValue;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::resource::ResourceDetector;
use opentelemetry_semantic_conventions as semconv;
use crate::detector::ec2::instance_metadata::EC2InstanceMetadataClient;

/// `EC2ResourceDetector` detects additional resource attributes from an AWS EC2 environment.
pub struct EC2ResourceDetector {
    client: Box<dyn instance_metadata::Client>
}

impl EC2ResourceDetector {
    pub fn new() -> Self { Self::default() }

    pub fn with_client(client: Box<dyn instance_metadata::Client>) -> Self {
        Self { client }
    }
}

impl Default for EC2ResourceDetector {
    fn default() -> Self {
        Self { client: Box::new(EC2InstanceMetadataClient::default()) }
    }
}

impl ResourceDetector for EC2ResourceDetector {
    fn detect(&self, timeout: Duration) -> Resource {
        let result = self.client.get_instance_identity_document(timeout);
        if result.is_err() {
            return Resource::empty();
        }

        let doc = result.unwrap();
        let attributes = [
            KeyValue::new(semconv::resource::CLOUD_PROVIDER, "aws"),
            KeyValue::new(semconv::resource::CLOUD_PLATFORM, "aws_ec2"),
            KeyValue::new(semconv::resource::CLOUD_ACCOUNT_ID, doc.account_id),
            KeyValue::new(semconv::resource::CLOUD_REGION, doc.region),
            KeyValue::new(semconv::resource::CLOUD_AVAILABILITY_ZONE, doc.availability_zone),
            KeyValue::new(semconv::resource::HOST_ID, doc.instance_id),
            KeyValue::new(semconv::resource::HOST_TYPE, doc.instance_type),
            KeyValue::new(semconv::resource::HOST_IMAGE_ID, doc.image_id),
        ];

        Resource::new(attributes)
    }
}

#[cfg(test)]
#[cfg(feature = "detector-aws-ec2")]
mod tests {
    use instance_metadata::EC2InstanceIdentityDocument;
    use super::*;

    struct TestClient {
        available: bool,
        document: EC2InstanceIdentityDocument
    }

    impl instance_metadata::Client for TestClient {
        fn get_instance_identity_document(&self, _: Duration) -> instance_metadata::Result<EC2InstanceIdentityDocument> {
            if self.available {
                Ok(self.document.clone())
            } else {
                Err(instance_metadata::Error::HttpRequestFailed("something went wrong".to_string()))
            }
        }
    }

    #[test]
    fn test_aws_ec2_detector() {
        let client = TestClient {
            available: true,
            document: EC2InstanceIdentityDocument {
                instance_id: "i-1234567890abcdef0".to_string(),
                account_id: "123456789012".to_string(),
                image_id: "ami-5fb8c835".to_string(),
                instance_type: "t2.micro".to_string(),
                architecture: "x86_64".to_string(),
                availability_zone: "eu-west-1a".to_string(),
                private_ip: "10.0.0.45".to_string(),
                region: "eu-west-1".to_string()
            }
        };

        let expected = Resource::new([
            KeyValue::new(semconv::resource::CLOUD_PROVIDER, "aws"),
            KeyValue::new(semconv::resource::CLOUD_PLATFORM, "aws_ec2"),
            KeyValue::new(semconv::resource::CLOUD_ACCOUNT_ID, "123456789012"),
            KeyValue::new(semconv::resource::CLOUD_REGION, "eu-west-1"),
            KeyValue::new(semconv::resource::CLOUD_AVAILABILITY_ZONE, "eu-west-1a"),
            KeyValue::new(semconv::resource::HOST_ID, "i-1234567890abcdef0"),
            KeyValue::new(semconv::resource::HOST_TYPE, "t2.micro"),
            KeyValue::new(semconv::resource::HOST_IMAGE_ID, "ami-5fb8c835"),
        ]);

        let detector = EC2ResourceDetector::with_client(Box::new(client));
        let got = detector.detect(Duration::from_secs(15));

        assert_eq!(expected, got)
    }

    #[test]
    fn test_aws_ec2_detector_returns_empty_when_error_retrieving_document() {
        let client = TestClient {
            available: false,
            document: EC2InstanceIdentityDocument::default()
        };

        let detector = EC2ResourceDetector::with_client(Box::new(client));
        let got = detector.detect(Duration::from_secs(15));

        assert_eq!(Resource::empty(), got)
    }
}