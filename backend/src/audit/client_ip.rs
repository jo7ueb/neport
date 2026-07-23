use axum::{async_trait, extract::{ConnectInfo, FromRequestParts}, http::request::Parts};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use crate::state::AppState;

/// Best-effort internet-facing client IP for the audit trail. Never rejects
/// a request - an audit log entry missing an IP must not be able to break
/// an otherwise-valid request.
///
/// Prefers `X-Real-IP`, which nginx.conf sets to a value it resolves and
/// controls end-to-end (see the `$true_client_ip` map there for why this is
/// trusted over `X-Forwarded-For` specifically on Cloud Run, where the
/// request may cross a second, undocumented proxy hop between the frontend
/// and backend Cloud Run services). Falls back to `X-Forwarded-For`'s first
/// entry (e.g. for local `cargo run` + a test that sets the header directly,
/// without nginx in front at all), then to the direct TCP peer.
pub struct ClientIp(pub Option<String>);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &Arc<AppState>) -> Result<Self, Infallible> {
        let ip = parts
            .headers
            .get("X-Real-IP")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.trim().to_string())
            .or_else(|| {
                parts
                    .headers
                    .get("X-Forwarded-For")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.split(',').next())
                    .map(|s| s.trim().to_string())
            })
            .or_else(|| parts.extensions.get::<ConnectInfo<SocketAddr>>().map(|ci| ci.0.ip().to_string()));

        Ok(ClientIp(ip))
    }
}
