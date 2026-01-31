use super::*;

#[test]
fn from_str_test() {
    assert_eq!(555, MemoryMetrics::from_str("555").unwrap().value);

    assert_eq!(100_000, MemoryMetrics::from_str("100KB").unwrap().value);
    assert_eq!(250_000_000_000, MemoryMetrics::from_str("250Gb").unwrap().value);

    assert_eq!(102_400, MemoryMetrics::from_str("100KiB").unwrap().value);
    assert_eq!(17_825_792, MemoryMetrics::from_str("17Mi").unwrap().value);

    assert_eq!(555_000_000_000, CpuMetrics::from_str("555").unwrap().value);

    assert_eq!(100_000_000, CpuMetrics::from_str("100m").unwrap().value);
    assert_eq!(100, CpuMetrics::from_str("100n").unwrap().value);
}

#[test]
fn add_test() {
    let expected = MemoryMetrics::from_str("640Ki").unwrap();
    let a = MemoryMetrics::from_str("512Ki").unwrap();
    let b = MemoryMetrics::from_str("128Ki").unwrap();
    assert_eq!(expected, a + b);

    let expected = MemoryMetrics::from_str("2560Ki").unwrap();
    let a = MemoryMetrics::from_str("512Ki").unwrap();
    let b = MemoryMetrics::from_str("2Mi").unwrap();
    assert_eq!(expected, a + b);
}

#[test]
fn display_test() {
    let a = MemoryMetrics::from_str("1Gi").unwrap();
    let b = MemoryMetrics::from_str("2Gi").unwrap();
    assert_eq!("3Gi", format!("{}", a + b));

    let a = MemoryMetrics::from_str("500GB").unwrap();
    let b = MemoryMetrics::from_str("500gb").unwrap();
    assert_eq!("1TB", format!("{}", a + b));

    let a = MemoryMetrics::from_str("128Mi").unwrap();
    let b = MemoryMetrics::from_str("2Gi").unwrap();
    assert_eq!("2176Mi", format!("{}", a + b));

    let a = MemoryMetrics::from_str("15").unwrap();
    let b = MemoryMetrics::from_str("5Mi").unwrap();
    assert_eq!("5242895B", format!("{}", a + b));

    let a = CpuMetrics::from_str("366455n").unwrap();
    let b = CpuMetrics::from_str("15m").unwrap();
    assert_eq!("15366455n", format!("{}", a + b));
    assert_eq!("15m", format!("{}", (a + b).millicores()));
}
