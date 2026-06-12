// SPDX-License-Identifier: Apache-2.0

//! Upstream OIDC provider discovery and Relying Party clients.

use std::collections::HashMap;
use std::sync::Arc;

use openidconnect::core::{
    CoreAuthenticationFlow, CoreProviderMetadata, CoreTokenResponse, CoreUserInfoClaims,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse,
};
use reqwest::Client;
use tracing::instrument;

use crate::config::{BrokerConfig, UpstreamIdpConfig};
use crate::error::BrokerError;

/// OpenID Connect client discovered from upstream provider metadata.
pub type DiscoveredClient = openidconnect::core::CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

/// A discovered upstream IdP client ready for authorization code + PKCE flows.
pub struct UpstreamIdp {
    /// Configured short name.
    pub name: String,
    /// OpenID Connect client for this upstream IdP.
    pub client: DiscoveredClient,
    /// Scopes requested during upstream authentication.
    pub scopes: Vec<String>,
    /// Claim mapping configuration.
    pub config: UpstreamIdpConfig,
}

/// Registry of discovered upstream IdP clients keyed by configured name.
pub struct UpstreamRegistry {
    idps: HashMap<String, Arc<UpstreamIdp>>,
    default_idp: Option<String>,
}

impl UpstreamRegistry {
    /// Discover all configured upstream IdPs on startup.
    #[instrument(skip(config, http_client))]
    pub async fn discover_all(
        config: &BrokerConfig,
        http_client: &Client,
    ) -> Result<Self, BrokerError> {
        let mut idps = HashMap::new();
        let mut default_idp = None;

        for idp_config in &config.upstream_idps {
            let idp = discover_idp(config, idp_config, http_client).await?;
            if default_idp.is_none() {
                default_idp = Some(idp.name.clone());
            }
            idps.insert(idp.name.clone(), Arc::new(idp));
        }

        if idps.is_empty() {
            return Err(BrokerError::Config(
                "at least one upstream_idps entry is required".to_string(),
            ));
        }

        Ok(Self { idps, default_idp })
    }

    /// Look up an upstream IdP by configured name.
    pub fn get(&self, name: &str) -> Result<Arc<UpstreamIdp>, BrokerError> {
        self.idps.get(name).cloned().ok_or(BrokerError::UnknownIdp)
    }

    /// Return the default upstream IdP when `/login` is called without a name.
    pub fn default(&self) -> Result<Arc<UpstreamIdp>, BrokerError> {
        let name = self.default_idp.as_ref().ok_or(BrokerError::UnknownIdp)?;
        self.get(name)
    }

    /// List configured upstream IdP names.
    pub fn names(&self) -> Vec<String> {
        self.idps.keys().cloned().collect()
    }
}

/// Authorization redirect URL components produced for an upstream login.
pub struct AuthorizationRequest {
    /// URL to redirect the browser to.
    pub auth_url: String,
    /// CSRF state stored in the RP session cookie.
    pub csrf_state: String,
    /// PKCE verifier stored in the RP session cookie.
    pub pkce_verifier: String,
    /// Nonce stored in the RP session cookie.
    pub nonce: String,
}

impl UpstreamIdp {
    /// Build an authorization request with PKCE for this upstream IdP.
    #[instrument(skip(self))]
    pub fn authorization_request(&self) -> Result<AuthorizationRequest, BrokerError> {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let mut request = self
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(pkce_challenge);

        for scope in &self.scopes {
            request = request.add_scope(Scope::new(scope.clone()));
        }

        let (auth_url, csrf_token, nonce) = request.url();

        Ok(AuthorizationRequest {
            auth_url: auth_url.to_string(),
            csrf_state: csrf_token.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
            nonce: nonce.secret().clone(),
        })
    }

    /// Exchange an authorization code for tokens and validate the upstream ID token.
    #[instrument(skip(self, http_client, code, pkce_verifier))]
    pub async fn exchange_code(
        &self,
        http_client: &Client,
        code: &str,
        pkce_verifier: &str,
        nonce: &str,
    ) -> Result<CoreTokenResponse, BrokerError> {
        let token_response = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|err| BrokerError::UpstreamOidc(err.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce_verifier.to_string()))
            .request_async(http_client)
            .await
            .map_err(|err| BrokerError::UpstreamOidc(err.to_string()))?;

        let id_token = token_response
            .id_token()
            .ok_or(BrokerError::AuthenticationFailed)?;
        id_token
            .claims(
                &self.client.id_token_verifier(),
                &Nonce::new(nonce.to_string()),
            )
            .map_err(|_| BrokerError::AuthenticationFailed)?;

        Ok(token_response)
    }

    /// Fetch upstream userinfo claims when an access token is available.
    #[instrument(skip(self, http_client, token_response))]
    pub async fn fetch_userinfo(
        &self,
        http_client: &Client,
        token_response: &CoreTokenResponse,
    ) -> Result<Option<CoreUserInfoClaims>, BrokerError> {
        let userinfo_request = self
            .client
            .user_info(token_response.access_token().to_owned(), None)
            .map_err(|err| BrokerError::UpstreamOidc(err.to_string()))?;

        userinfo_request
            .request_async(http_client)
            .await
            .map(Some)
            .map_err(|err| BrokerError::UpstreamOidc(err.to_string()))
    }
}

async fn discover_idp(
    broker: &BrokerConfig,
    config: &UpstreamIdpConfig,
    http_client: &Client,
) -> Result<UpstreamIdp, BrokerError> {
    let issuer = IssuerUrl::new(config.issuer.clone())
        .map_err(|err| BrokerError::Config(format!("invalid issuer `{}`: {err}", config.name)))?;
    let metadata = CoreProviderMetadata::discover_async(issuer, http_client)
        .await
        .map_err(|err| {
            BrokerError::UpstreamOidc(format!("discovery failed for `{}`: {err}", config.name))
        })?;

    let client_secret = BrokerConfig::upstream_client_secret(config).map_err(|err| {
        BrokerError::Config(format!("missing `{}`: {err}", config.client_secret_env))
    })?;

    let redirect = RedirectUrl::new(broker.callback_url())
        .map_err(|err| BrokerError::Config(format!("invalid callback URL: {err}")))?;

    let client =
        CoreClientBuilder::from_metadata(metadata, &config.client_id, &client_secret, redirect)?;

    Ok(UpstreamIdp {
        name: config.name.clone(),
        client,
        scopes: config.scopes.clone(),
        config: config.clone(),
    })
}

struct CoreClientBuilder;

impl CoreClientBuilder {
    fn from_metadata(
        metadata: CoreProviderMetadata,
        client_id: &str,
        client_secret: &str,
        redirect: RedirectUrl,
    ) -> Result<DiscoveredClient, BrokerError> {
        Ok(openidconnect::core::CoreClient::from_provider_metadata(
            metadata,
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
        )
        .set_redirect_uri(redirect))
    }
}

/// Build the shared HTTP client used for upstream OIDC discovery and token exchange.
pub fn build_http_client() -> Result<Client, BrokerError> {
    Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| BrokerError::Internal(format!("HTTP client: {err}")))
}
