use std::time::Duration;

use serde::Deserialize;

use crate::net::error::NetError;

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceInfo {
    #[serde(default)]
    pub product: String,
    #[serde(default)]
    pub firmware_version: String,
    #[serde(default)]
    pub hostname: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mismatch {
    pub offset: usize,
    pub expected: u8,
    pub actual: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyReport {
    pub address: u16,
    pub written: usize,
    pub mismatch: Option<Mismatch>,
}

#[derive(Debug, Clone)]
pub struct UltimateClient {
    base: String,
    password: Option<String>,
    http: reqwest::Client,
}

impl UltimateClient {
    pub fn new(host: &str, password: Option<String>) -> Self {
        let host = host.trim().trim_end_matches('/');
        let base = if host.starts_with("http://") || host.starts_with("https://") {
            host.to_string()
        } else {
            format!("http://{host}")
        };
        // Two separate bounds, because they guard two different failures.
        //
        // CONNECT_TIMEOUT is the one that matters day to day: a wrong-but-live
        // IP (a firewall silently dropping the SYN rather than refusing) hangs
        // in the TCP handshake, and without a bound the caller waits out the OS
        // retry schedule — ~75s on macOS. 5s is long enough for any device on a
        // LAN to answer and short enough that a typo in the host field costs a
        // pause, not a coffee break.
        //
        // TIMEOUT covers the whole request once connected, and is deliberately
        // far more generous: `readmem` accepts a length up to 65536, and a large
        // DMA read from real hardware over wifi should not race a five-second
        // clock. A connected-but-slow device is a different situation from an
        // unreachable one and deserves a different budget.
        //
        // `build()` is documented as only failing on TLS backend init, which
        // this app's TLS-free config never exercises — but `new` is infallible
        // by contract, so fall back rather than panic if it ever does. The
        // fallback client has no timeouts; that is strictly better than crashing
        // and is unreachable in practice.
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { base, password, http }
    }

    fn with_password(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.password {
            Some(p) => req.header("X-Password", p),
            None => req,
        }
    }

    pub async fn write_mem(&self, address: u16, data: &[u8]) -> Result<(), NetError> {
        if address as usize + data.len() > 0x1_0000 {
            return Err(NetError::WouldWrap { address, len: data.len() });
        }

        let url = format!("{}/v1/machine:writemem", self.base);
        let req = self
            .http
            .post(&url)
            .query(&[("address", format!("{address:04X}"))])
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec());

        let resp = self
            .with_password(req)
            .send()
            .await
            .map_err(|e| NetError::Transport(e.to_string()))?;

        Self::parse_envelope(resp).await?;
        Ok(())
    }

    pub async fn read_mem(&self, address: u16, length: u32) -> Result<Vec<u8>, NetError> {
        if address as usize + length as usize > 0x1_0000 {
            return Err(NetError::WouldWrap { address, len: length as usize });
        }

        let url = format!("{}/v1/machine:readmem", self.base);
        let req = self.http.get(&url).query(&[
            ("address", format!("{address:04X}")),
            ("length", length.to_string()),
        ]);

        let resp = self
            .with_password(req)
            .send()
            .await
            .map_err(|e| NetError::Transport(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            return Err(NetError::Http { status: status.as_u16() });
        }
        let bytes = resp.bytes().await.map_err(|e| NetError::Transport(e.to_string()))?;
        Ok(bytes.to_vec())
    }

    pub async fn version(&self) -> Result<String, NetError> {
        let url = format!("{}/v1/version", self.base);
        let resp = self
            .with_password(self.http.get(&url))
            .send()
            .await
            .map_err(|e| NetError::Transport(e.to_string()))?;

        let value = Self::parse_envelope(resp).await?;
        Ok(value.get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string())
    }

    pub async fn info(&self) -> Result<DeviceInfo, NetError> {
        let url = format!("{}/v1/info", self.base);
        let resp = self
            .with_password(self.http.get(&url))
            .send()
            .await
            .map_err(|e| NetError::Transport(e.to_string()))?;

        let value = Self::parse_envelope(resp).await?;
        serde_json::from_value(value).map_err(|e| NetError::Transport(e.to_string()))
    }

    pub async fn write_and_verify(
        &self,
        address: u16,
        data: &[u8],
    ) -> Result<VerifyReport, NetError> {
        self.write_mem(address, data).await?;
        let back = self.read_mem(address, data.len() as u32).await?;

        let mismatch = data
            .iter()
            .zip(back.iter())
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(offset, (expected, actual))| Mismatch {
                offset,
                expected: *expected,
                actual: *actual,
            })
            .or_else(|| {
                (back.len() < data.len()).then(|| Mismatch {
                    offset: back.len(),
                    expected: data[back.len()],
                    actual: 0,
                })
            });

        Ok(VerifyReport { address, written: data.len(), mismatch })
    }

    /// The device answers HTTP 200 with a JSON `errors` array even on failure,
    /// so a 200 status alone is not success. Every endpoint that returns a JSON
    /// envelope (writemem, version, info) is checked here; `read_mem` returns
    /// raw bytes on success and is handled separately.
    async fn parse_envelope(resp: reqwest::Response) -> Result<serde_json::Value, NetError> {
        let status = resp.status();
        if !status.is_success() {
            return Err(NetError::Http { status: status.as_u16() });
        }
        let text = resp.text().await.map_err(|e| NetError::Transport(e.to_string()))?;
        if text.trim().is_empty() {
            return Ok(serde_json::Value::Null);
        }
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| NetError::Transport(e.to_string()))?;
        if let Some(arr) = value.get("errors").and_then(|v| v.as_array())
            && !arr.is_empty()
        {
            let errors: Vec<String> = arr
                .iter()
                .map(|e| e.as_str().map(str::to_string).unwrap_or_else(|| e.to_string()))
                .collect();
            return Err(NetError::Api { errors });
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_bytes, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn ok_body() -> serde_json::Value {
        serde_json::json!({ "errors": [] })
    }

    #[tokio::test]
    async fn write_mem_sends_address_query_and_binary_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/machine:writemem"))
            .and(query_param("address", "C000"))
            .and(header("content-type", "application/octet-stream"))
            .and(body_bytes(vec![0xA9u8, 0x08]))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .expect(1)
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        client.write_mem(0xC000, &[0xA9, 0x08]).await.expect("should succeed");
    }

    #[tokio::test]
    async fn address_is_formatted_as_four_uppercase_hex_digits() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(query_param("address", "0400"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .expect(1)
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        client.write_mem(0x0400, &[0x08]).await.expect("should succeed");
    }

    #[tokio::test]
    async fn http_200_with_non_empty_errors_array_is_a_failure() {
        // This is the critical contract: the device returns 200 even on failure.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": ["address out of range"]
            })))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let err = client.write_mem(0xC000, &[0x00]).await.expect_err("must not treat 200 as success");
        match err {
            NetError::Api { errors } => assert_eq!(errors, vec!["address out of range".to_string()]),
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn password_header_is_sent_when_configured() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(header("x-password", "hunter2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .expect(1)
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), Some("hunter2".into()));
        client.write_mem(0xC000, &[0x00]).await.expect("should succeed");
    }

    #[tokio::test]
    async fn forbidden_maps_to_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let err = client.write_mem(0xC000, &[0x00]).await.expect_err("403 is an error");
        assert!(matches!(err, NetError::Http { status: 403 }), "got {err:?}");
    }

    #[tokio::test]
    async fn unreachable_host_maps_to_transport_error() {
        // Port 1 on localhost: nothing listens there.
        let client = UltimateClient::new("http://127.0.0.1:1", None);
        let err = client.write_mem(0xC000, &[0x00]).await.expect_err("should fail");
        assert!(matches!(err, NetError::Transport(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn read_mem_returns_binary_body_unchanged() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/machine:readmem"))
            .and(query_param("address", "0400"))
            .and(query_param("length", "4"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![8u8, 9, 32, 32]))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let got = client.read_mem(0x0400, 4).await.expect("should succeed");
        assert_eq!(got, vec![8, 9, 32, 32]);
    }

    #[tokio::test]
    async fn wrap_past_ffff_is_rejected_without_issuing_a_request() {
        let server = MockServer::start().await;
        // No mock mounted: any outgoing request would fail the test.
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .expect(0)
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let err = client.write_mem(0xFFFE, &[1, 2, 3, 4]).await.expect_err("should reject locally");
        assert!(matches!(err, NetError::WouldWrap { .. }), "got {err:?}");
    }

    #[tokio::test]
    async fn write_and_verify_reports_success_when_readback_matches() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v1/machine:writemem"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/machine:readmem"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0xA9u8, 0x08]))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let report = client.write_and_verify(0xC000, &[0xA9, 0x08]).await.expect("should succeed");
        assert_eq!(report.written, 2);
        assert_eq!(report.address, 0xC000);
        assert!(report.mismatch.is_none());
    }

    #[tokio::test]
    async fn write_and_verify_reports_the_first_differing_byte() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0xA9u8, 0xFF]))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let report = client.write_and_verify(0xC000, &[0xA9, 0x08]).await.expect("call succeeds");
        let m = report.mismatch.expect("should report a mismatch");
        assert_eq!((m.offset, m.expected, m.actual), (1, 0x08, 0xFF));
    }

    #[tokio::test]
    async fn info_parses_device_fields() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "product": "Commodore 64 Ultimate",
                "firmware_version": "3.12",
                "hostname": "ultimate"
            })))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let info = client.info().await.expect("should succeed");
        assert_eq!(info.product, "Commodore 64 Ultimate");
        assert_eq!(info.firmware_version, "3.12");
    }

    #[tokio::test]
    async fn version_extracts_the_version_field_from_the_envelope() {
        // Pins the field-name extraction: version() must pull "version" out of
        // the JSON envelope, not return the raw body text.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "version": "0.1",
                "errors": []
            })))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let version = client.version().await.expect("should succeed");
        assert_eq!(version, "0.1");
    }

    #[tokio::test]
    async fn version_http_200_with_non_empty_errors_array_is_a_failure() {
        // Same 200-with-errors contract as write_mem, asserted directly on version().
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/version"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": ["not ready"]
            })))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let err = client.version().await.expect_err("must not treat 200 as success");
        match err {
            NetError::Api { errors } => assert_eq!(errors, vec!["not ready".to_string()]),
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn info_http_200_with_non_empty_errors_array_is_a_failure() {
        // Realistic case: a bad password rejected with 200 + errors rather than
        // a 403. Also covers the #[serde(default)] trap: without the envelope
        // check, an error-only body would silently deserialize into an
        // empty-but-Ok DeviceInfo.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": ["bad password"]
            })))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let err = client.info().await.expect_err("must not treat 200 as success");
        match err {
            NetError::Api { errors } => assert_eq!(errors, vec!["bad password".to_string()]),
            other => panic!("expected Api error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn non_string_errors_elements_are_not_silently_treated_as_success() {
        // A non-empty "errors" array must be a failure regardless of element
        // type. Element type must not decide the branch, only array emptiness.
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "errors": [5]
            })))
            .mount(&server)
            .await;

        let client = UltimateClient::new(&server.uri(), None);
        let err = client.write_mem(0xC000, &[0x00]).await.expect_err("must not treat 200 as success");
        match err {
            NetError::Api { errors } => assert_eq!(errors, vec!["5".to_string()]),
            other => panic!("expected Api error, got {other:?}"),
        }
    }
}
