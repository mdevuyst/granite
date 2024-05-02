/// Parse a list of socket addresses given as "ip:port" strings (e.g., "0.0.0.0:80") into a list of
/// ports.
pub fn collect_ports(addrs: &[String]) -> Vec<u16> {
    addrs
        .iter()
        .map(|addr| addr.split(':').last().unwrap().parse().unwrap())
        .collect()
}
