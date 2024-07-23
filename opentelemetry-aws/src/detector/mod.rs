#[cfg(feature = "detector-aws-lambda")]
mod lambda;
#[cfg(feature = "detector-aws-ec2")]
mod ec2;

#[cfg(feature = "detector-aws-lambda")]
pub use lambda::LambdaResourceDetector;
#[cfg(feature = "detector-aws-ec2")]
pub use ec2::EC2ResourceDetector;
