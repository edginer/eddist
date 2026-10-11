use aws_sdk_s3::{Client, error::SdkError, operation::get_object::GetObjectError};

use super::AppService;

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalService(Client, String);

impl KakoThreadRetrievalService {
    pub fn new(client: Client, bucket_name: String) -> Self {
        Self(client, bucket_name)
    }
}

#[async_trait::async_trait]
impl AppService<KakoThreadRetrievalServiceInput, Vec<u8>, KakoThreadRetrievalError>
    for KakoThreadRetrievalService
{
    async fn execute(
        &self,
        input: KakoThreadRetrievalServiceInput,
    ) -> Result<Vec<u8>, KakoThreadRetrievalError> {
        let key = format!("{}/dat/{}.dat", input.board_key, input.thread_number);
        match self.0.get_object().bucket(&self.1).key(&key).send().await {
            Ok(output) => {
                let bytes = output
                    .body
                    .collect()
                    .await
                    .map_err(anyhow::Error::from)?
                    .into_bytes()
                    .to_vec();
                Ok(bytes)
            }
            Err(SdkError::ServiceError(e)) if matches!(e.err(), GetObjectError::NoSuchKey(_)) => {
                Err(KakoThreadRetrievalError::NotFound)
            }
            Err(err) => {
                log::error!(
                    "Error retrieving kako thread: {err:?}, path: {}/dat/{}.dat",
                    input.board_key,
                    input.thread_number
                );
                Err(anyhow::anyhow!("Error retrieving thread: {err:?}").into())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KakoThreadRetrievalError {
    #[error("archived thread not found")]
    NotFound,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: String,
}
