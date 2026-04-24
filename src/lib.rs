mod client;
mod commands;
mod err;
mod licensed;
mod machine;

use std::sync::Arc;

use client::KeygenClient;
use err::Error;
use licensed::*;
use machine::Machine;
use tauri::{
    plugin::{Builder as PluginBuilder, TauriPlugin},
    Manager, Runtime,
};
use tokio::sync::Mutex;

pub type Result<T> = std::result::Result<T, Error>;

/// Closure type used to resolve a device fingerprint at plugin setup time.
/// See [`Builder::fingerprint`].
pub type FingerprintResolver =
    Arc<dyn Fn() -> std::result::Result<String, String> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct Builder {
    pub custom_domain: Option<String>,
    pub api_url: Option<String>,
    pub account_id: Option<String>,
    pub verify_key: String,
    pub version_header: Option<String>,
    pub cache_lifetime: i64, // in minutes
    /// Optional caller-provided fingerprint resolver. When unset the plugin
    /// falls back to `machine_uid` (desktop-only). Supplying a resolver is
    /// required for mobile targets where `machine_uid` is unavailable.
    pub fingerprint: Option<FingerprintResolver>,
}

impl Builder {
    pub fn new(account_id: impl Into<String>, verify_key: impl Into<String>) -> Self {
        Self {
            custom_domain: None,
            api_url: Some("https://api.keygen.sh".into()),
            account_id: Some(account_id.into()),
            verify_key: verify_key.into(),
            version_header: None,
            cache_lifetime: 240,
            fingerprint: None,
        }
    }

    pub fn with_custom_domain(
        custom_domain: impl Into<String>,
        verify_key: impl Into<String>,
    ) -> Self {
        Self {
            custom_domain: Some(custom_domain.into()),
            account_id: None,
            api_url: None,
            verify_key: verify_key.into(),
            version_header: None,
            cache_lifetime: 240,
            fingerprint: None,
        }
    }

    /// Supply a closure that returns the device fingerprint. Invoked once
    /// during plugin setup, so it can depend on other plugins (e.g. a
    /// keychain backend) that are registered beforehand.
    ///
    /// When this is **not** called the plugin falls back to `machine_uid`,
    /// which only works on desktop targets (Linux/macOS/Windows). Mobile
    /// callers must supply a resolver.
    pub fn fingerprint<F>(mut self, resolver: F) -> Self
    where
        F: Fn() -> std::result::Result<String, String> + Send + Sync + 'static,
    {
        self.fingerprint = Some(Arc::new(resolver));
        self
    }

    pub fn api_url(mut self, api_url: impl Into<String>) -> Self {
        if self.custom_domain.is_none() {
            self.api_url = Some(api_url.into());
        }
        self
    }

    pub fn version_header(mut self, version_header: impl Into<String>) -> Self {
        self.version_header = Some(version_header.into());
        self
    }

    pub fn cache_lifetime(mut self, cache_lifetime: i64) -> Self {
        self.cache_lifetime = cache_lifetime.clamp(60, 1440);
        self
    }

    pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
        PluginBuilder::new("keygen")
            .invoke_handler(tauri::generate_handler![
                commands::get_license,
                commands::get_license_key,
                commands::validate_key,
                commands::activate,
                commands::checkout_machine,
                commands::reset_license,
                commands::reset_license_key,
            ])
            .setup(move |app, _api| {
                // get app info
                let app_name = app.package_info().name.clone();
                let app_version = app.package_info().version.to_string();

                // Resolve fingerprint: caller-supplied closure takes priority;
                // otherwise fall back to `machine_uid` (desktop-only).
                let fingerprint = match &self.fingerprint {
                    Some(resolver) => resolver().map_err(|e| {
                        Box::<dyn std::error::Error>::from(format!(
                            "tauri-plugin-keygen: fingerprint resolver failed: {e}"
                        ))
                    })?,
                    #[cfg(not(any(target_os = "ios", target_os = "android")))]
                    None => machine_uid::get().unwrap_or_default(),
                    #[cfg(any(target_os = "ios", target_os = "android"))]
                    None => {
                        return Err(Box::<dyn std::error::Error>::from(
                            "tauri-plugin-keygen: a fingerprint resolver is required on iOS/Android; \
                             call Builder::fingerprint(...) before Builder::build().",
                        ));
                    }
                };

                // init machine
                let machine = Machine::new(app_name, app_version, fingerprint);

                // init keygen client
                let keygen_client = KeygenClient::new(
                    self.custom_domain,
                    self.api_url,
                    self.account_id,
                    self.verify_key,
                    self.version_header,
                    self.cache_lifetime,
                    machine.user_agent.clone(),
                );

                // init state
                match LicensedState::load(app, &keygen_client, &machine) {
                    Ok(licensed_state) => {
                        app.manage(Mutex::new(licensed_state));
                    }
                    Err(err) => {
                        dbg!(err);
                        app.manage(Mutex::new(LicensedState::default()));
                    }
                }
                app.manage(Mutex::new(machine));
                app.manage(Mutex::new(keygen_client));

                Ok(())
            })
            .build()
    }
}
