pub fn collect_ports(addrs: &[String]) -> Vec<u16> {
    addrs
        .iter()
        .map(|addr| addr.split(':').last().unwrap().parse().unwrap())
        .collect()
}
