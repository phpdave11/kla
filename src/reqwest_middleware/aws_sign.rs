use std::{path::Path, time::SystemTime};

use aws_sigv4::sign::v4 as aws_sigv4;
use http::Extensions;
use reqwest::{Request, Response};
use reqwest_middleware::{Middleware, Next, Result};

pub struct AWSSignBody {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub service: String,
}

impl AWSSignBody {
    pub fn new_from_file<P: AsRef<Path>>(path: P) -> AWSSignBody {
        todo!()
    }
}

const ALGORITHM: &'static str = "AWS4-HMAC-SHA256";

#[async_trait::async_trait]
impl Middleware for AWSSignBody {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let datestamp = SystemTime::now();
        let signing_key = aws_sigv4::generate_signing_key(
            &self.secret_key,
            datestamp.clone(),
            &self.region,
            &self.service,
        );

        let canonical_request = format!(
            "{}\n{}\n\n{}\n{}\n{}",
            req.method(),
            "/", // Simplified - extract path from URL in production
            req.headers(),
            req,
            payload_hash
        );

        let signature = aws_sigv4::calculate_signature(
            signing_key,
            http::Request::from(sig_request)
                .body()
                .map(|v| v.as_bytes())
                .flatten()
                .unwrap_or_default(),
        );

        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            datestamp, self.region, self.service
        );

        format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            ALGORITHM, self.access_key, credential_scope, signed_headers, signature
        );

        let res = next.run(req, extensions).await;

        res
    }
}
