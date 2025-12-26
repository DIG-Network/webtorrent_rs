use webtorrent::blocklist::Blocklist;
use std::net::IpAddr;

#[tokio::test]
async fn test_blocklist_new() {
    let blocklist = Blocklist::new();
    assert!(!blocklist.is_blocked("127.0.0.1".parse().unwrap()).await);
}

#[tokio::test]
async fn test_blocklist_add_ip() {
    let blocklist = Blocklist::new();
    let ip: IpAddr = "192.168.1.1".parse().unwrap();
    
    blocklist.add_ip(ip).await;
    assert!(blocklist.is_blocked(ip).await);
    
    blocklist.remove_ip(ip).await;
    assert!(!blocklist.is_blocked(ip).await);
}

#[tokio::test]
async fn test_blocklist_add_cidr() {
    let blocklist = Blocklist::new();
    let network: IpAddr = "192.168.1.0".parse().unwrap();
    let prefix_len = 24;
    
    blocklist.add_cidr(network, prefix_len).await.unwrap();
    
    // Test IPs in the CIDR range
    assert!(blocklist.is_blocked("192.168.1.1".parse().unwrap()).await);
    assert!(blocklist.is_blocked("192.168.1.100".parse().unwrap()).await);
    assert!(blocklist.is_blocked("192.168.1.255".parse().unwrap()).await);
    
    // Test IPs outside the CIDR range
    assert!(!blocklist.is_blocked("192.168.2.1".parse().unwrap()).await);
    assert!(!blocklist.is_blocked("10.0.0.1".parse().unwrap()).await);
    
    blocklist.remove_cidr(network, prefix_len).await;
    assert!(!blocklist.is_blocked("192.168.1.1".parse().unwrap()).await);
}

#[tokio::test]
async fn test_blocklist_load_from_string() {
    let blocklist = Blocklist::new();
    let blocklist_data = "192.168.1.1\n10.0.0.0/8\n172.16.0.0/12";
    
    blocklist.load_from_string(blocklist_data).await.unwrap();
    
    assert!(blocklist.is_blocked("192.168.1.1".parse().unwrap()).await);
    assert!(blocklist.is_blocked("10.0.0.1".parse().unwrap()).await);
    assert!(blocklist.is_blocked("172.16.0.1".parse().unwrap()).await);
    assert!(!blocklist.is_blocked("192.168.1.2".parse().unwrap()).await);
}

#[tokio::test]
async fn test_blocklist_empty() {
    let blocklist = Blocklist::new();
    assert!(!blocklist.is_blocked("127.0.0.1".parse().unwrap()).await);
    assert!(!blocklist.is_blocked("192.168.1.1".parse().unwrap()).await);
}

