pub fn is_challenge(explicit: bool, body: &str) -> bool {
    let b = body.to_ascii_lowercase();
    explicit
        || ((b.contains("/cdn-cgi/challenge-platform/")
            || b.contains("_cf_chl_opt")
            || b.contains("cf-chl-"))
            && (b.contains("<title>just a moment")
                || b.contains("<title>attention required!")
                || b.contains("checking your browser")
                || b.contains("enable javascript and cookies to continue")))
}

use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct SolverConfig {
    pub endpoint: Option<String>,
    pub timeout_seconds: u64,
}
impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            endpoint: None,
            timeout_seconds: 120,
        }
    }
}
#[derive(Default)]
pub struct State {
    blocked: AtomicBool,
    pub(crate) token: tokio::sync::Mutex<Option<(String, std::time::Instant)>>,
}
impl State {
    pub fn reset(&self) {
        self.blocked.store(false, Ordering::Relaxed)
    }
}
pub type Cancel = Arc<dyn Fn() -> bool + Send + Sync>;
pub type Route = Arc<dyn Fn(&Url) -> Result<Url, String> + Send + Sync>;
pub struct Http {
    pub(crate) client: Client,
    pub(crate) solver_client: Client,
    pub(crate) state: Arc<State>,
    pub(crate) solver: SolverConfig,
    pub(crate) cancelled: Cancel,
    pub(crate) route: Route,
}
impl Http {
    pub fn new(
        client: Client,
        solver_client: Client,
        state: Arc<State>,
        solver: SolverConfig,
        cancelled: Cancel,
        route: Route,
    ) -> Self {
        Self {
            client,
            solver_client,
            state,
            solver,
            cancelled,
            route,
        }
    }

    pub fn check(&self) -> Result<(), String> {
        if (self.cancelled)() {
            Err("Cancelled: a newer tag source request replaced this one.".into())
        } else {
            Ok(())
        }
    }
    pub async fn cancellable<F: std::future::Future>(
        &self,
        future: F,
    ) -> Result<F::Output, String> {
        self.check()?;
        let cancelled = async {
            loop {
                if (self.cancelled)() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(25)).await
            }
        };
        tokio::select! {result=future=>{self.check()?;Ok(result)},_=cancelled=>Err("Cancelled: a newer tag source request replaced this one.".into())}
    }
    async fn body(&self, mut response: reqwest::Response, max: usize) -> Result<String, String> {
        if response.content_length().is_some_and(|n| n > max as u64) {
            return Err("Catalog response exceeds size limit".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = self
            .cancellable(response.chunk())
            .await?
            .map_err(|_| "Could not read catalog response")?
        {
            if bytes.len() + chunk.len() > max {
                return Err("Catalog response exceeds size limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        String::from_utf8(bytes).map_err(|_| "Catalog response is not UTF-8".into())
    }
    pub async fn post_form(&self, url: &Url, form: &[(&str, &str)]) -> Result<String, String> {
        let url = (self.route)(url)?;
        let response = self
            .cancellable(self.client.post(url.clone()).form(form).send())
            .await?
            .map_err(|_| "Could not contact catalog token service")?;
        if response.url().origin() != url.origin() {
            return Err("Catalog token service redirected outside its origin".into());
        }
        if !response.status().is_success() {
            return Err(format!(
                "Catalog token service HTTP {}",
                response.status().as_u16()
            ));
        }
        self.body(response, 1024 * 1024).await
    }
    pub async fn get(
        &self,
        url: &Url,
        bearer: Option<&str>,
        solver_allowed: bool,
    ) -> Result<String, String> {
        self.check()?;
        // Solvers only receive public GET targets; never bearer tokens or local identities.
        if solver_allowed && bearer.is_some() {
            return Err("Authenticated catalog requests cannot use a solver".into());
        }
        let target = (self.route)(url)?;
        if !solver_allowed || !self.state.blocked.load(Ordering::Relaxed) {
            let mut request = self.client.get(target.clone());
            if let Some(token) = bearer {
                request = request.bearer_auth(token)
            }
            let response = self
                .cancellable(request.send())
                .await?
                .map_err(|_| "Could not contact catalog service")?;
            if response.url().origin() != target.origin() {
                return Err("Catalog redirected outside its origin".into());
            }
            let explicit = response
                .headers()
                .get("cf-mitigated")
                .is_some_and(|h| h == "challenge");
            let status = response.status();
            let body = if explicit {
                String::new()
            } else {
                self.body(response, 8 * 1024 * 1024).await?
            };
            if !is_challenge(explicit, &body) {
                if !status.is_success() {
                    return Err(format!("Catalog HTTP {}", status.as_u16()));
                }
                return Ok(body);
            }
            if !solver_allowed {
                return Err("Catalog returned an unresolved challenge".into());
            }
            self.state.blocked.store(true, Ordering::Relaxed);
        }
        let endpoint=self.solver.api_url()?.ok_or("Traxsource requires a FlareSolverr-compatible solver; configure it in Tag Sources settings")?;
        for attempt in 0..2 {
            match self.solve(&endpoint, &target).await {
                Err(SolveError::Transient) if attempt == 0 => {
                    self.cancellable(tokio::time::sleep(std::time::Duration::from_millis(250)))
                        .await?;
                }
                Err(SolveError::Transient) => {
                    return Err(
                        "Solver timed out or is temporarily unavailable; retry the lookup".into(),
                    )
                }
                Err(SolveError::Permanent(message)) => return Err(message),
                Ok(body) => return Ok(body),
            }
        }
        unreachable!()
    }
    async fn solve(&self, endpoint: &Url, target: &Url) -> Result<String, SolveError> {
        self.check().map_err(SolveError::Permanent)?;
        let request=self.solver_client.post(endpoint.clone()).timeout(std::time::Duration::from_secs(self.solver.timeout_seconds+10)).json(&serde_json::json!({"cmd":"request.get","url":target.as_str(),"maxTimeout":self.solver.timeout_seconds*1000}));
        let response = self
            .cancellable(request.send())
            .await
            .map_err(SolveError::Permanent)?
            .map_err(|_| SolveError::Transient)?;
        if response.url() != endpoint {
            return Err(SolveError::Permanent(
                "Solver redirect is not allowed".into(),
            ));
        }
        let status = response.status();
        if [429, 502, 503, 504].contains(&status.as_u16()) {
            return Err(SolveError::Transient);
        }
        if !status.is_success() {
            return Err(SolveError::Permanent(format!(
                "Solver HTTP {}",
                status.as_u16()
            )));
        }
        let body = self
            .body(response, 16 * 1024 * 1024)
            .await
            .map_err(SolveError::Permanent)?;
        let envelope: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
            SolveError::Permanent("Invalid FlareSolverr-compatible response".into())
        })?;
        if envelope["status"].as_str() != Some("ok") {
            let message = envelope["message"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if [
                "timeout",
                "timed out",
                "deadline",
                "pool saturated",
                "pool exhausted",
            ]
            .iter()
            .any(|s| message.contains(s))
            {
                return Err(SolveError::Transient);
            }
            return Err(SolveError::Permanent(
                "Solver could not retrieve the catalog page; check the solver service".into(),
            ));
        }
        let solution = &envelope["solution"];
        let final_url = solution["url"]
            .as_str()
            .and_then(|s| Url::parse(s).ok())
            .ok_or_else(|| SolveError::Permanent("Solver returned an invalid final URL".into()))?;
        if target.origin() != final_url.origin() {
            return Err(SolveError::Permanent(
                "Solver redirected outside the catalog origin".into(),
            ));
        }
        let status = solution["status"].as_u64().unwrap_or(0);
        if !(200..300).contains(&status) {
            return Err(SolveError::Permanent(format!(
                "Solved catalog HTTP {status}"
            )));
        }
        let body = solution["response"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 8 * 1024 * 1024)
            .ok_or_else(|| {
                SolveError::Permanent("Solver returned empty or oversized catalog content".into())
            })?;
        let explicit = solution["headers"].as_object().is_some_and(|headers| {
            headers.iter().any(|(key, value)| {
                key.eq_ignore_ascii_case("cf-mitigated") && value.as_str() == Some("challenge")
            })
        });
        if is_challenge(explicit, body) {
            return Err(SolveError::Permanent(
                "Solver returned an unresolved challenge".into(),
            ));
        }
        Ok(body.into())
    }
}

impl SolverConfig {
    pub fn api_url(&self) -> Result<Option<Url>, String> {
        if !(1..=300).contains(&self.timeout_seconds) {
            return Err("Solver timeout must be between 1 and 300 seconds".into());
        }
        let Some(endpoint) = self
            .endpoint
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            return Ok(None);
        };
        let mut url = Url::parse(endpoint).map_err(|_| "Invalid solver URL")?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("Solver URL must use HTTP or HTTPS without credentials, query parameters or a fragment".into());
        }
        let path = url.path().trim_end_matches('/').to_owned();
        if path.ends_with("/v1") {
            url.set_path(&path)
        } else {
            url.set_path(&format!("{path}/v1"))
        }
        Ok(Some(url))
    }
}
enum SolveError {
    Transient,
    Permanent(String),
}
