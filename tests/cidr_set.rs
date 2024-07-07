use bitstring_trees::iter::iter_between;
use cidr::{
	AnyIpCidr,
	Ipv4Cidr,
};

#[test]
fn test_cidr_empty() {
	let set = bitstring_trees::set::Set::<AnyIpCidr>::new();
	assert_eq!(
		set.iter_full().collect::<Vec<_>>(),
		vec![(AnyIpCidr::Any, false)]
	);
}

#[test]
fn test_cidr() {
	let mut set = bitstring_trees::set::Set::<AnyIpCidr>::new();
	set.insert("192.168.10.0/24".parse().unwrap());
	set.insert("224.0.0.0/4".parse().unwrap());
	set.insert("8000::/1".parse().unwrap());
	assert_eq!(
		set.iter_full().collect::<Vec<_>>(),
		vec![
			("0.0.0.0/1".parse().unwrap(), false),
			("128.0.0.0/2".parse().unwrap(), false),
			("192.0.0.0/9".parse().unwrap(), false),
			("192.128.0.0/11".parse().unwrap(), false),
			("192.160.0.0/13".parse().unwrap(), false),
			("192.168.0.0/21".parse().unwrap(), false),
			("192.168.8.0/23".parse().unwrap(), false),
			("192.168.10.0/24".parse().unwrap(), true),
			("192.168.11.0/24".parse().unwrap(), false),
			("192.168.12.0/22".parse().unwrap(), false),
			("192.168.16.0/20".parse().unwrap(), false),
			("192.168.32.0/19".parse().unwrap(), false),
			("192.168.64.0/18".parse().unwrap(), false),
			("192.168.128.0/17".parse().unwrap(), false),
			("192.169.0.0/16".parse().unwrap(), false),
			("192.170.0.0/15".parse().unwrap(), false),
			("192.172.0.0/14".parse().unwrap(), false),
			("192.176.0.0/12".parse().unwrap(), false),
			("192.192.0.0/10".parse().unwrap(), false),
			("193.0.0.0/8".parse().unwrap(), false),
			("194.0.0.0/7".parse().unwrap(), false),
			("196.0.0.0/6".parse().unwrap(), false),
			("200.0.0.0/5".parse().unwrap(), false),
			("208.0.0.0/4".parse().unwrap(), false),
			("224.0.0.0/4".parse().unwrap(), true),
			("240.0.0.0/4".parse().unwrap(), false),
			("::/1".parse().unwrap(), false),
			("8000::/1".parse().unwrap(), true),
		]
	);
}

#[test]
fn test_fill_uncovered() {
	assert_eq!(
		iter_between::<AnyIpCidr>(None, Some("8000::/1".parse().unwrap())).collect::<Vec<_>>(),
		vec!["0.0.0.0/0".parse().unwrap(), "::/1".parse().unwrap(),],
	);

	assert_eq!(
		iter_between::<AnyIpCidr>(
			Some("240.0.0.0/4".parse().unwrap()),
			Some("8000::/1".parse().unwrap())
		)
		.collect::<Vec<_>>(),
		vec!["::/1".parse().unwrap(),],
	);

	assert_eq!(
		iter_between::<AnyIpCidr>(Some("240.0.0.0/4".parse().unwrap()), None).collect::<Vec<_>>(),
		vec!["::/0".parse().unwrap(),],
	);

	assert_eq!(
		iter_between::<Ipv4Cidr>(None, Some("240.0.0.0/4".parse().unwrap())).collect::<Vec<_>>(),
		vec![
			"0.0.0.0/1".parse().unwrap(),
			"128.0.0.0/2".parse().unwrap(),
			"192.0.0.0/3".parse().unwrap(),
			"224.0.0.0/4".parse().unwrap(),
		],
	);
}

#[test]
fn test_fill_uncovered2() {
	assert_eq!(
		iter_between::<AnyIpCidr>(
			Some("192.168.10.0/24".parse().unwrap()),
			Some("240.0.0.0/4".parse().unwrap())
		)
		.collect::<Vec<_>>(),
		vec![
			"192.168.11.0/24".parse().unwrap(),
			"192.168.12.0/22".parse().unwrap(),
			"192.168.16.0/20".parse().unwrap(),
			"192.168.32.0/19".parse().unwrap(),
			"192.168.64.0/18".parse().unwrap(),
			"192.168.128.0/17".parse().unwrap(),
			"192.169.0.0/16".parse().unwrap(),
			"192.170.0.0/15".parse().unwrap(),
			"192.172.0.0/14".parse().unwrap(),
			"192.176.0.0/12".parse().unwrap(),
			"192.192.0.0/10".parse().unwrap(),
			"193.0.0.0/8".parse().unwrap(),
			"194.0.0.0/7".parse().unwrap(),
			"196.0.0.0/6".parse().unwrap(),
			"200.0.0.0/5".parse().unwrap(),
			"208.0.0.0/4".parse().unwrap(),
			"224.0.0.0/4".parse().unwrap(),
		],
	);
}

#[test]
fn goto() {
	let mut set = bitstring_trees::set::Set::<AnyIpCidr>::new();
	set.insert("192.168.10.0/24".parse().unwrap()); // IPV4 b1100_0000 b1010_1000 b0000_1010
	set.insert("224.0.0.0/4".parse().unwrap()); // IPV4 b1110_0000
	set.insert("8000::/1".parse().unwrap());
	// -> nodes should be:
	// * "any"
	//   * IPv4 b11
	//     * leaf 192.168.10.0/24
	//     * leaf 224.0.0.0/4
	//   * leaf 8000::/1

	assert!(set.contains(&"192.168.10.0/24".parse().unwrap()));
	assert!(set.contains(&"192.168.10.0/25".parse().unwrap()));
	assert!(set.contains(&"192.168.10.128/25".parse().unwrap()));

	assert!(set.contains(&"224.0.0.0/4".parse().unwrap()));
	assert!(set.contains(&"224.0.0.0/5".parse().unwrap()));
	assert!(set.contains(&"232.0.0.0/5".parse().unwrap()));

	assert!(set.contains(&"8000::/1".parse().unwrap()));
	assert!(set.contains(&"8000::/2".parse().unwrap()));
	assert!(set.contains(&"c000::/2".parse().unwrap()));

	// "any" is "", IPv4 b11 is "011"
	// make sure all bits in the second one are checked
	assert!(!set.contains(&"40a8:0a00::/24".parse().unwrap())); // "1", IPv6
	assert!(!set.contains(&"0.168.10.0/24".parse().unwrap()));
	assert!(!set.contains(&"128.168.10.0/24".parse().unwrap()));

	set.remove("8000::/1".parse().unwrap());
	assert!(!set.contains(&"8000::/1".parse().unwrap()));
	// -> nodes should be:
	// * IPv4 b11
	//   * leaf 192.168.10.0/24
	//   * leaf 224.0.0.0/4

	// again check all bits (this time when looking at root node)
	assert!(!set.contains(&"80a8:0a00::/24".parse().unwrap())); // "1", IPv6
	assert!(!set.contains(&"0.168.10.0/24".parse().unwrap()));
	assert!(!set.contains(&"128.168.10.0/24".parse().unwrap()));
}
