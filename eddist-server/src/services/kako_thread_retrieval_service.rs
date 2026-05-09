use aws_sdk_s3::{Client, error::SdkError};

use super::AppService;

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalService(Client, String);

impl KakoThreadRetrievalService {
    pub fn new(client: Client, bucket_name: String) -> Self {
        Self(client, bucket_name)
    }
}

#[async_trait::async_trait]
impl AppService<KakoThreadRetrievalServiceInput, Vec<u8>> for KakoThreadRetrievalService {
    async fn execute(&self, input: KakoThreadRetrievalServiceInput) -> anyhow::Result<Vec<u8>> {
        let key = format!("{}/dat/{}.dat", input.board_key, input.thread_number);
        match self.0.get_object().bucket(&self.1).key(&key).send().await {
            Ok(output) => {
                let bytes = output.body.collect().await?.into_bytes().to_vec();
                Ok(bytes)
            }
            Err(SdkError::ServiceError(e)) if e.raw().status().as_u16() == 404 => {
                Err(anyhow::anyhow!("Thread not found"))
            }
            Err(err) => {
                log::error!(
                    "Error retrieving kako thread: {err:?}, path: {}/dat/{}.dat",
                    input.board_key,
                    input.thread_number
                );
                Err(anyhow::anyhow!("Error retrieving thread: {err:?}"))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: String,
}
