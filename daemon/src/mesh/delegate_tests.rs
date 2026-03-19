#[cfg(test)]
mod tests {
    use crate::mesh::delegate::*;
    use crate::mesh::peers::PeerConfig;
    use std::time::Duration;

    fn test_peer(ssh_alias: &str, user: &str, ip: &str) -> PeerConfig {
        PeerConfig {
            ssh_alias: ssh_alias.to_owned(),
            user: user.to_owned(),
            os: "macos".to_owned(),
            tailscale_ip: ip.to_owned(),
            dns_name: "worker.example.ts.net".to_owned(),
            capabilities: vec!["claude".to_owned()],
            role: "worker".to_owned(),
            status: "active".to_owned(),
            mac_address: None,
            gh_account: None,
            runners: None,
            runner_paths: None,
        }
    }

    #[test]
    fn parse_tokens_bracket_format() {
        let output = "starting...\n[tokens: 4200]\ndone";
        assert_eq!(parse_tokens_from_output(output), 4200);
    }

    #[test]
    fn parse_tokens_equals_format() {
        let output = "output line\ntokens_used=9001\n";
        assert_eq!(parse_tokens_from_output(output), 9001);
    }

    #[test]
    fn parse_tokens_none_returns_zero() {
        assert_eq!(parse_tokens_from_output("no token info here"), 0);
    }

    #[test]
    fn worktree_branch_format() {
        assert_eq!(worktree_branch(671, "T6-01"), "delegate/plan-671/T6-01");
    }

    #[test]
    fn ssh_destination_prefers_alias() {
        let peer = test_peer("mac-dev-ts", "alice", "100.64.0.1");
        assert_eq!(ssh_destination(&peer), "mac-dev-ts");
    }

    #[test]
    fn ssh_destination_fallback_to_ip() {
        let peer = test_peer("", "bob", "100.64.0.2");
        assert_eq!(ssh_destination(&peer), "bob@100.64.0.2");
    }

    #[test]
    fn delegate_timeout_default() {
        std::env::remove_var("DELEGATE_TIMEOUT");
        assert_eq!(delegate_timeout(), Duration::from_secs(1800));
    }
}
