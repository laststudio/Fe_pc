use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct OssFormConfig {
    pub host: String,
    pub dir: String,
    pub access_id: String,
    pub policy: String,
    pub signature: String,
    pub callback: String,
}

pub struct OssUploader;

impl OssUploader {
    /// 使用 ETS 返回的表单凭据直接 POST 到阿里云 OSS
    #[inline(never)]
    pub fn upload_form(
        config: &OssFormConfig,
        file_name: &str,
        content: &[u8],
    ) -> Result<String, String> {
        let (boundary, body) = crate::themida_tiger_red!({
            let boundary = format!("------------------------{}", Uuid::new_v4().simple());
            let mut body = Vec::with_capacity(content.len() + 1024);

            let fields = [
                ("name", file_name),
                ("key", &format!("{}{}", config.dir, file_name)),
                ("OSSAccessKeyId", &config.access_id),
                ("policy", &config.policy),
                ("signature", &config.signature),
                ("callback", &config.callback),
            ];

            for (name, val) in fields {
                body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                );
                body.extend_from_slice(val.as_bytes());
                body.extend_from_slice(b"\r\n");
            }

            // 写入文件数据
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            body.extend_from_slice(
                format!("Content-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\n")
                    .as_bytes(),
            );
            body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
            body.extend_from_slice(content);
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

            (boundary, body)
        });

        let host = if config.host.starts_with("http") {
            config.host.clone()
        } else {
            format!("https://{}", config.host)
        };

        let resp_res = ureq::post(&host)
            .timeout(Duration::from_secs(30))
            .set(
                "Content-Type",
                &format!("multipart/form-data; boundary={boundary}"),
            )
            .set("Content-Length", &body.len().to_string())
            .set("User-Agent", "libcurl-agent/1.0")
            .send_bytes(&body);

        match resp_res {
            Ok(resp) => {
                let status = resp.status();
                if status == 200 || status == 204 {
                    let cdn_prefix = host.trim_end_matches('/');
                    let dir = config.dir.trim_matches('/');
                    let full_path = if dir.is_empty() {
                        file_name.to_string()
                    } else {
                        format!("{dir}/{file_name}")
                    };
                    Ok(format!("{cdn_prefix}/{full_path}"))
                } else {
                    Err(format!("OSS 上传返回异常状态码: {status}"))
                }
            }
            Err(ureq::Error::Status(status, resp)) => {
                let err_text = resp.into_string().unwrap_or_default();
                Err(format!("OSS 上传失败 (HTTP {status}): {err_text}"))
            }
            Err(e) => Err(format!("OSS 上传网络异常: {e}")),
        }
    }
}
