// INI parser and serialiser for ~/.claude/config/peers.conf.
//
// Format:
//   [section_name]
//   key=value
//
// Sections: [mesh] for global config, [peer_name] for each peer.
// Comment lines start with '#'. Blank lines are ignored.

use std::collections::BTreeMap;

use super::types::{PeerConfig, PeersError};

// ── Parsing helpers ──────────────────────────────────────────────────────────

pub(super) fn require(
    map: &BTreeMap<String, String>,
    key: &str,
    peer: &str,
) -> Result<String, PeersError> {
    map.get(key)
        .cloned()
        .ok_or_else(|| PeersError::MissingField {
            peer: peer.to_owned(),
            field: key.to_owned(),
        })
}

pub(super) fn parse_capabilities(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn build_peer(
    name: &str,
    kv: &BTreeMap<String, String>,
) -> Result<PeerConfig, PeersError> {
    Ok(PeerConfig {
        ssh_alias: require(kv, "ssh_alias", name)?,
        user: require(kv, "user", name)?,
        os: require(kv, "os", name)?,
        tailscale_ip: require(kv, "tailscale_ip", name)?,
        dns_name: require(kv, "dns_name", name)?,
        capabilities: parse_capabilities(&require(kv, "capabilities", name)?),
        role: require(kv, "role", name)?,
        status: kv
            .get("status")
            .cloned()
            .unwrap_or_else(|| "active".to_owned()),
        mac_address: kv.get("mac_address").cloned(),
        gh_account: kv.get("gh_account").cloned(),
        runners: kv.get("runners").and_then(|v| v.parse::<u32>().ok()),
        runner_paths: kv.get("runner_paths").cloned(),
    })
}

// ── INI parser ────────────────────────────────────────────────────────────────

fn flush_section(
    section: &Option<String>,
    kv: &BTreeMap<String, String>,
    secret: &mut String,
    peers: &mut BTreeMap<String, PeerConfig>,
) -> Result<(), PeersError> {
    if let Some(name) = section {
        if name == "mesh" {
            if let Some(s) = kv.get("shared_secret") {
                *secret = s.clone();
            }
        } else {
            let cfg = build_peer(name, kv)?;
            peers.insert(name.clone(), cfg);
        }
    }
    Ok(())
}

pub fn parse_ini(text: &str) -> Result<(String, BTreeMap<String, PeerConfig>), PeersError> {
    let mut shared_secret = String::new();
    let mut peers: BTreeMap<String, PeerConfig> = BTreeMap::new();
    let mut current_section: Option<String> = None;
    let mut current_kv: BTreeMap<String, String> = BTreeMap::new();

    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            flush_section(
                &current_section,
                &current_kv,
                &mut shared_secret,
                &mut peers,
            )?;
            current_section = Some(line[1..line.len() - 1].to_owned());
            current_kv = BTreeMap::new();
        } else if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_owned();
            let val = line[eq + 1..].trim().to_owned();
            current_kv.insert(key, val);
        } else {
            return Err(PeersError::Parse {
                line: lineno + 1,
                msg: format!("unexpected content: {line}"),
            });
        }
    }
    flush_section(
        &current_section,
        &current_kv,
        &mut shared_secret,
        &mut peers,
    )?;
    Ok((shared_secret, peers))
}

// ── Serialiser ────────────────────────────────────────────────────────────────

fn caps_str(caps: &[String]) -> String {
    caps.join(",")
}

pub fn peer_to_ini(name: &str, p: &PeerConfig) -> String {
    let mut out = format!(
        "[{name}]\nssh_alias={}\nuser={}\nos={}\ntailscale_ip={}\ndns_name={}\ncapabilities={}\nrole={}\nstatus={}\n",
        p.ssh_alias,
        p.user,
        p.os,
        p.tailscale_ip,
        p.dns_name,
        caps_str(&p.capabilities),
        p.role,
        p.status,
    );
    if let Some(ref mac) = p.mac_address {
        out.push_str(&format!("mac_address={mac}\n"));
    }
    if let Some(ref gh) = p.gh_account {
        out.push_str(&format!("gh_account={gh}\n"));
    }
    if let Some(r) = p.runners {
        out.push_str(&format!("runners={r}\n"));
    }
    if let Some(ref rp) = p.runner_paths {
        out.push_str(&format!("runner_paths={rp}\n"));
    }
    out
}
