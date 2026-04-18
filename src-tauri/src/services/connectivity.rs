use crate::models::error::{AppError, AppResult};
use std::net::ToSocketAddrs;
use tokio::{
    net::TcpStream,
    time::{timeout, Duration},
};

#[derive(Debug, Clone)]
pub struct TcpTargetProbe {
    pub target: String,
    pub reachable: bool,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct BootstrapReachabilityReport {
    pub attempted_brokers: usize,
    pub reachable_brokers: usize,
    pub probes: Vec<TcpTargetProbe>,
}

pub async fn preflight_bootstrap_servers(
    bootstrap_servers: &str,
) -> AppResult<BootstrapReachabilityReport> {
    let targets = normalize_bootstrap_servers(bootstrap_servers)?;
    let attempted = targets.len();
    let mut reachable = 0usize;
    let mut probes = Vec::with_capacity(attempted);

    for target in targets {
        let probe = probe_tcp_target(&target).await;

        if probe.reachable {
            reachable += 1;
        }

        probes.push(probe);
    }

    Ok(BootstrapReachabilityReport {
        attempted_brokers: attempted,
        reachable_brokers: reachable,
        probes,
    })
}

pub async fn probe_tcp_target(target: &str) -> TcpTargetProbe {
    match target.to_socket_addrs() {
        Ok(addresses) => {
            let addresses = addresses.collect::<Vec<_>>();
            if addresses.is_empty() {
                return TcpTargetProbe {
                    target: target.to_string(),
                    reachable: false,
                    detail: "DNS 解析未返回任何可连接地址。".to_string(),
                };
            }

            for address in &addresses {
                let attempt = timeout(Duration::from_secs(2), TcpStream::connect(address)).await;

                if let Ok(Ok(_stream)) = attempt {
                    return TcpTargetProbe {
                        target: target.to_string(),
                        reachable: true,
                        detail: format!(
                            "已解析 {} 个地址，并成功连接到 {}。",
                            addresses.len(),
                            address
                        ),
                    };
                }
            }

            TcpTargetProbe {
                target: target.to_string(),
                reachable: false,
                detail: format!(
                    "已解析 {} 个地址，但所有 TCP 连接均失败或超时。",
                    addresses.len()
                ),
            }
        }
        Err(error) => TcpTargetProbe {
            target: target.to_string(),
            reachable: false,
            detail: format!("DNS 解析失败：{error}"),
        },
    }
}

pub fn normalize_bootstrap_servers(bootstrap_servers: &str) -> AppResult<Vec<String>> {
    let targets = bootstrap_servers
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            if value.contains(':') {
                value.to_string()
            } else {
                format!("{value}:9092")
            }
        })
        .collect::<Vec<_>>();

    if targets.is_empty() {
        return Err(AppError::Validation(
            "bootstrap servers must include at least one host".to_string(),
        ));
    }

    Ok(targets)
}
