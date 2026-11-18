//! Maps a pool the device mines on to that pool's public page for the mining address.
//!
//! The link is a convenience for the pool the device is already configured for, not a
//! recommendation of any pool. Nothing here talks to the network.

/// Pools that publish per-address statistics, as their stratum hosts and the page for an address.
///
/// Only pools that take a bare Bitcoin address as the stratum user can be listed, since the page
/// is derived from that address. Hosts must match exactly, so self-hosted instances and lookalike
/// domains never get a link.
const POOLS: &[(&[&str], &str)] = &[
    (
        &[
            "solo.ckpool.org",
            "eusolo.ckpool.org",
            "ausolo.ckpool.org",
            "sgsolo.ckpool.org",
        ],
        "https://stats.ckpool.org/users/{address}",
    ),
    (&["mine.ocean.xyz"], "https://ocean.xyz/stats/{address}"),
    (
        &["public-pool.io"],
        "https://web.public-pool.io/#/app/{address}",
    ),
    (
        &["stratum.btcpowlab-pool.com"],
        "https://btcpowlab-pool.com/miner/{address}",
    ),
];

const BECH32_CHARSET: &str = "qpzry9x8gf2tvdw0s3jn54khce6mua7l";
const BASE58_CHARSET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// Returns the pool's page for the address mining on `host` as `user`, if the pool is known and
/// the user starts with a plausible Bitcoin mainnet address.
pub fn pool_dashboard(host: &str, user: &str) -> Option<String> {
    let (_, template) = POOLS
        .iter()
        .find(|(hosts, _)| hosts.iter().any(|known| known.eq_ignore_ascii_case(host)))?;

    // Pools take an optional worker name after the address, separated by `.` or `_`. Neither
    // character can appear in an address.
    let address = user.split(['.', '_']).next().unwrap_or_default();

    is_plausible_address(address).then(|| template.replace("{address}", address))
}

/// Whether `address` has the shape of a Bitcoin mainnet address. Checksums are not verified: the
/// goal is to never build a link out of something that is not an address, not to validate it.
fn is_plausible_address(address: &str) -> bool {
    let all_in = |text: &str, charset: &str| text.chars().all(|c| charset.contains(c));

    if let Some(data) = address.strip_prefix("bc1") {
        // Between a P2WPKH (42 characters) and a P2WSH or P2TR (62 characters) address.
        return (42..=62).contains(&address.len()) && all_in(data, BECH32_CHARSET);
    }

    (address.starts_with('1') || address.starts_with('3'))
        && (25..=34).contains(&address.len())
        && all_in(address, BASE58_CHARSET)
}

#[cfg(test)]
mod test {
    use super::*;

    const SEGWIT: &str = "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq";
    const TAPROOT: &str = "bc1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297";
    const LEGACY: &str = "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa";
    const SCRIPT: &str = "3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy";

    #[test]
    fn links_each_known_pool() {
        let cases = [
            ("solo.ckpool.org", "https://stats.ckpool.org/users/"),
            ("eusolo.ckpool.org", "https://stats.ckpool.org/users/"),
            ("ausolo.ckpool.org", "https://stats.ckpool.org/users/"),
            ("sgsolo.ckpool.org", "https://stats.ckpool.org/users/"),
            ("mine.ocean.xyz", "https://ocean.xyz/stats/"),
            ("public-pool.io", "https://web.public-pool.io/#/app/"),
            (
                "stratum.btcpowlab-pool.com",
                "https://btcpowlab-pool.com/miner/",
            ),
        ];

        for (host, prefix) in cases {
            assert_eq!(
                pool_dashboard(host, SEGWIT),
                Some(format!("{prefix}{SEGWIT}")),
                "{host}"
            );
        }
    }

    #[test]
    fn accepts_every_mainnet_address_type() {
        for address in [SEGWIT, TAPROOT, LEGACY, SCRIPT] {
            assert_eq!(
                pool_dashboard("mine.ocean.xyz", address),
                Some(format!("https://ocean.xyz/stats/{address}")),
            );
        }
    }

    #[test]
    fn drops_the_worker_name() {
        let expected = Some(format!("https://ocean.xyz/stats/{SEGWIT}"));

        for user in [
            format!("{SEGWIT}.bitaxe"),
            format!("{SEGWIT}_bitaxe"),
            format!("{SEGWIT}.rig.one"),
            format!("{SEGWIT}."),
        ] {
            assert_eq!(pool_dashboard("mine.ocean.xyz", &user), expected, "{user}");
        }
    }

    #[test]
    fn matches_the_host_regardless_of_case() {
        assert!(pool_dashboard("Solo.CKPool.org", SEGWIT).is_some());
    }

    #[test]
    fn ignores_unknown_and_lookalike_hosts() {
        for host in [
            "",
            "example.com",
            "192.168.1.10",
            "ckpool.org",
            "solo.ckpool.org.evil.example",
            "evil-solo.ckpool.org",
            "solo.ckpool.org:3333",
            "stratum+tcp://solo.ckpool.org",
            "solo.ckpool.org/",
            " solo.ckpool.org",
            "pool.public-pool.io",
            "stratum.btcpowlab-pool.co",
        ] {
            assert_eq!(pool_dashboard(host, SEGWIT), None, "{host}");
        }
    }

    #[test]
    fn ignores_users_that_are_not_mainnet_addresses() {
        let too_long = format!("{TAPROOT}q");
        let users = [
            "",
            "worker",
            ".worker",
            "account.worker",
            "bc1qexample",
            // Testnet and regtest.
            "tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxpjzsx",
            "bcrt1qw508d6qejxtdg4y5r3zarvary0c5xw7kygt080",
            "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn",
            // Characters outside the address alphabets.
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdb",
            "BC1QAR0SRRR7XFKVY5L643LYDNW9RE59GTZZWF5MDQ",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfN0",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7Div/../",
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5md?x=1",
            // Wrong length.
            "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5md",
            "1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa1A1zP1eP",
            too_long.as_str(),
        ];

        for user in users {
            assert_eq!(pool_dashboard("mine.ocean.xyz", user), None, "{user}");
        }
    }
}
