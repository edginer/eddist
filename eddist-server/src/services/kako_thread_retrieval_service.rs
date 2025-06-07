use s3::Bucket;

use super::AppService;

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalService(Bucket);

impl KakoThreadRetrievalService {
    pub fn new(bucket: Bucket) -> Self {
        Self(bucket)
    }
}

#[async_trait::async_trait]
impl AppService<KakoThreadRetrievalServiceInput, Vec<u8>> for KakoThreadRetrievalService {
    async fn execute(&self, input: KakoThreadRetrievalServiceInput) -> anyhow::Result<Vec<u8>> {
        match self
            .0
            .get_object(format!(
                "{}/dat/{}.dat",
                input.board_key, input.thread_number
            ))
            .await
        {
            Ok(obj) => Ok(obj.bytes().to_vec()),
            Err(err) => match err {
                s3::error::S3Error::HttpFailWithBody(404, _) => {
                    Err(anyhow::anyhow!("Thread not found"))
                }
                _ => {
                    log::error!(
                        "Error retrieving kako thread: {err:?}, path: {}/dat/{}.dat",
                        input.board_key,
                        input.thread_number
                    );
                    Err(anyhow::anyhow!("Error retrieving thread: {err:?}"))
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: String,
}
