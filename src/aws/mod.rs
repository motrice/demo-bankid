extern crate rusoto_core;
extern crate rusoto_credential;
extern crate rusoto_s3;
extern crate futures;

use rusoto_core::{Region};
use rusoto_s3::S3Client;
use rusoto_s3::S3;
use rusoto_s3::util::PreSignedRequest;
use rusoto_credential::{AwsCredentials, EnvironmentProvider, ProvideAwsCredentials};
use futures::future::Future;

#[derive(Clone)]
pub struct Aws {
    credentials: AwsCredentials,
    region: Region
}

impl Aws {
    pub fn new() -> Aws {
        let credentials = EnvironmentProvider::default().credentials().wait().unwrap();
        let region = Region::EuNorth1;
        Aws{credentials: credentials, region: region}
    }

    pub fn test_put_s3(&self) {

        let s3 = S3Client::new(Region::EuNorth1);

        match s3.put_object(rusoto_s3::PutObjectRequest{
            bucket: String::from("motrice-insignia"),
            key: String::from("12345"),
            body: Some(String::from("hej").into_bytes().into()),
            ..Default::default()
        }).sync() {
            Ok(val) => println!("ok"),
            Err(err) => println!("err")
        }
    }

    pub fn put_s3_signed_url(&self, filename: &str) -> String {
        let req = rusoto_s3::PutObjectRequest {
            bucket: String::from("motrice-insignia"),
            key: filename.to_owned(),
            ..Default::default()
        };
        req.get_presigned_url(&rusoto_core::Region::EuNorth1, &self.credentials, &Default::default())
    }

}

