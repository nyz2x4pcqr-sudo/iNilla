//! **The launcher meets the login.** A server profile becomes benilla's realmlist (and realm name,
//! and the world-address override); a saved account becomes a login attempt through the login
//! screen's own submit path, so the connecting dialog, the queue and every refusal read exactly as
//! for a typed login.
//!
//! The **realm address override** is read by the network thread when it dials the world server
//! (`net::io`, one call to [`world_address`]): the realm list's address is kept unless the active
//! profile names a host to use instead — the LAN/public NAT fix a phone that moves between Wi-Fi
//! and cellular needs.

use std::sync::RwLock;

use bevy::prelude::*;

use inilla_core::profiles::{Account, ServerProfile};

use super::Inilla;
use crate::client_state::ClientState;
use crate::cvars::Cvars;

/// The active profile's override, for the network thread.
static WORLD_OVERRIDE: RwLock<Option<ServerProfile>> = RwLock::new(None);

/// The world address to dial for a realm the list advertised at `listed`.
pub(crate) fn world_address(listed: String) -> String {
    let guard = WORLD_OVERRIDE.read().ok();
    match guard.as_ref().and_then(|g| g.as_ref()) {
        Some(p) => {
            let addr = p.world_address(&listed);
            if addr != listed {
                bevy::log::info!("inilla: realm address {listed} → {addr} (profile override)");
            }
            addr
        }
        None => listed,
    }
}

/// A login the launcher asked for, waiting for the login screen.
#[derive(Resource, Default)]
pub(crate) struct LoginQueue {
    pending: Option<(String, String)>,
    /// The profile whose realmlist was last applied.
    applied: Option<String>,
}

impl LoginQueue {
    pub(crate) fn request(&mut self, user: &str, pass: &str) {
        self.pending = Some((user.to_string(), pass.to_string()));
    }
}

/// Make `server` the active one: realmlist, realm name, override.
pub(crate) fn activate(server: &ServerProfile, cvars: &mut Cvars) {
    cvars.set(crate::realmlist::CVAR_REALMLIST, &server.realmlist());
    if let Some(realm) = server
        .realm_name
        .as_deref()
        .filter(|r| !r.trim().is_empty())
    {
        cvars.set("realmName", realm.trim());
    }
    if let Ok(mut g) = WORLD_OVERRIDE.write() {
        *g = Some(server.clone());
    }
    info!(
        "inilla: server {:?} — realmlist {}{}",
        server.name,
        server.realmlist(),
        server
            .realm_override
            .as_deref()
            .filter(|o| !o.trim().is_empty())
            .map(|o| format!(", world host {o}"))
            .unwrap_or_default()
    );
}

/// The password saved for `account`, if the player chose to remember it.
pub(crate) fn saved_password(account: &Account) -> Option<String> {
    inilla_platform::keychain::get(&account.secret_key())
}

pub(crate) struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LoginQueue>()
            .add_systems(Update, (apply_active_server, submit_queued_login).chain());
    }
}

/// Keep benilla's realmlist on the launcher's current server.
fn apply_active_server(
    state: Res<Inilla>,
    mut queue: ResMut<LoginQueue>,
    mut cvars: ResMut<Cvars>,
) {
    let Some(server) = state.launcher.current_server() else {
        return;
    };
    let fingerprint = format!(
        "{}|{}|{:?}|{:?}",
        server.id,
        server.realmlist(),
        server.realm_override,
        server.realm_name
    );
    if queue.applied.as_deref() == Some(fingerprint.as_str()) {
        return;
    }
    activate(server, &mut cvars);
    queue.applied = Some(fingerprint);
}

/// Hand a queued login to the stock login screen's own verb once that screen is up: the launcher
/// runs `DefaultServerLogin(account, password)` in the glue VM with the call host lent, exactly
/// the call `AccountLogin_Login` makes when the button is pressed, so the glue manager's guards,
/// its status dialog, every failure text and the realm list behave as for a typed login. The
/// verb itself refuses while a logon is already running.
fn submit_queued_login(
    state: Res<State<ClientState>>,
    mut queue: ResMut<LoginQueue>,
    mut hosted_vm: crate::script_calls::HostedVm,
) {
    if *state.get() != ClientState::Login || queue.pending.is_none() {
        return;
    }
    hosted_vm.run(|script| {
        let Some(script) = script else {
            return;
        };
        // The stock login frame is shown once GlueXML has loaded and put the login screen up.
        let ready = matches!(
            script
                .eval_plain("return AccountLogin ~= nil and AccountLogin:IsVisible() == 1")
                .as_deref(),
            Ok([benilla_ui::script::plain::PlainValue::Bool(true), ..])
        );
        if !ready {
            return;
        }
        let Some((user, pass)) = queue.pending.take() else {
            return;
        };
        info!("inilla: logging in as {user}");
        let chunk = format!(
            "DefaultServerLogin({}, {})",
            inilla_core::lua::quote(&user),
            inilla_core::lua::quote(&pass)
        );
        if let Err(e) = script.run(&chunk) {
            warn!("inilla: the login call failed: {e}");
        }
    });
}
