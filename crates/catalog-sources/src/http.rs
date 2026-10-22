//! Catalog transport: direct requests first, the Cloudflare solver when a host blocks them.
use crate::solver::{self, BlockedHosts, SolveError};
pub use crate::solver::{is_challenge, SolverConfig};
use reqwest::{Client, Url};
use std::sync::Arc;

pub struct State {
    blocked: Arc<BlockedHosts>,
    pub(crate) token: tokio::sync::Mutex<Option<(String, std::time::Instant)>>,
}
impl Default for State {
    fn default() -> Self {
        Self::shared(Arc::default())
    }
}
impl State {
    /// A state that records Cloudflare blocks in a registry shared with other transports.
    pub fn shared(blocked: Arc<BlockedHosts>) -> Self {
        Self {
            blocked,
            token: Default::default(),
        }
    }
    pub fn reset(&self) {
        self.blocked.clear()
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
        // After a block, another direct request to the same host would only repeat it.
        if self.state.blocked.contains(&target) {
            return self.after_block(&target, solver_allowed).await;
        }
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
        let status = response.status();
        let headers = response.headers().clone();
        let explicit = headers
            .get("cf-mitigated")
            .is_some_and(|h| h == "challenge");
        let body = if explicit {
            String::new()
        } else {
            self.body(response, 8 * 1024 * 1024).await?
        };
        if solver::is_cloudflare_block(status.as_u16(), &headers, &body) {
            self.state.blocked.mark(&target);
            return self.after_block(&target, solver_allowed).await;
        }
        if !status.is_success() {
            return Err(format!("Catalog HTTP {}", status.as_u16()));
        }
        Ok(body)
    }
    /// The host blocks direct requests: use the solver when the request may, otherwise stop.
    async fn after_block(&self, target: &Url, solver_allowed: bool) -> Result<String, String> {
        if !solver_allowed {
            return Err(format!(
                "{} is blocking TagGroove (Cloudflare). Requests that carry credentials cannot use a solver; try again later or use Retry direct access in Options → Tag Sources → Cloudflare solver.",
                target.host_str().unwrap_or("The site")
            ));
        }
        let api = self
            .solver
            .api_url()?
            .ok_or_else(|| solver::no_solver_message(target.host_str().unwrap_or("The site")))?;
        for attempt in 0..2 {
            self.check()?;
            let solved = self
                .cancellable(solver::solve(
                    &self.solver_client,
                    &api,
                    self.solver.timeout_seconds,
                    target,
                ))
                .await?;
            match solved {
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
                Ok(page) => return Ok(page.body),
            }
        }
        unreachable!()
    }
}
