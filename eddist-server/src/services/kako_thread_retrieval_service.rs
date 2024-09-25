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
        let obj = self
            .0
            .get_object(format!(
                "{}/dat/{}.dat",
                input.board_key, input.thread_number
            ))
            .await?;
        Ok(obj.bytes().to_vec())
    }
}

#[derive(Debug, Clone)]
pub struct KakoThreadRetrievalServiceInput {
    pub board_key: String,
    pub thread_number: String,
}
